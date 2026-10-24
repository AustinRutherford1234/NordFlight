//! Same atmosphere vertex colors as earth.rs, evaluated without rebuilding meshes.
use crate::{
    earth::{CloudCover, Haze, Surface, region::Region},
    physics::RADIUS,
};
use bevy::{
    asset::{load_internal_asset, uuid_handle},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::{Shader, ShaderRef},
};

const SHADER: Handle<Shader> = uuid_handle!("f156a6c4-f706-4f0e-86af-ffb84cbd1b06");
#[derive(Clone, Copy, Debug, ShaderType, PartialEq)]
pub struct Parameters {
    pub eye: Vec4,
    pub sun: Vec4,
    // radius, height, rim strength, mode (0 haze, 1 cloud, 2 surface)
    pub layer: Vec4,
}
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct EarthExtension {
    #[uniform(100)]
    pub parameters: Parameters,
    #[texture(101)]
    #[sampler(102)]
    pub nordic: Option<Handle<Image>>,
}
impl MaterialExtension for EarthExtension {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }
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
pub type EarthMaterial = ExtendedMaterial<StandardMaterial, EarthExtension>;
pub struct EarthGpuPlugin;
impl Plugin for EarthGpuPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER, "earth_vertex.wgsl", Shader::from_wgsl);
        app.add_plugins(MaterialPlugin::<EarthMaterial>::default())
            .add_systems(PostStartup, convert)
            .add_systems(
                PostUpdate,
                material_changes.run_if(crate::performance::enabled),
            );
    }
}
fn material_changes(mut changes: MessageReader<AssetEvent<EarthMaterial>>) {
    let count = changes
        .read()
        .filter(|e| matches!(e, AssetEvent::Modified { .. }))
        .count();
    crate::performance::EARTH_MATERIAL_CHANGES
        .fetch_add(count as u64, std::sync::atomic::Ordering::Relaxed);
}
fn convert(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
    mut extended: ResMut<Assets<EarthMaterial>>,
    mut images: ResMut<Assets<Image>>,
    layers: Query<
        (
            Entity,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            Option<&Haze>,
            Option<&CloudCover>,
            Option<&Region>,
        ),
        Or<(With<Haze>, With<CloudCover>, With<Surface>)>,
    >,
) {
    if std::env::var_os("NORD_EARTH_CPU").is_some() {
        return;
    }
    let nordic = crate::earth::region::texture(&mut images);
    // Globe patches share one base material and must share one Earth
    // material too, or every patch would be updated separately.
    let mut shared = std::collections::HashMap::new();
    for (entity, mesh, material, haze, cloud, region) in &layers {
        let Some(base) = materials.get(&material.0) else {
            continue;
        };
        if let Some(existing) = shared.get(&material.0.id()) {
            let m = meshes.get_mut(&mesh.0).unwrap();
            let colors: Vec<_> = (0..m.count_vertices())
                .map(|i| [1., 1., 1., region.map_or(1., |r| r.0[i])])
                .collect();
            m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
            commands.entity(entity).remove::<MeshMaterial3d<StandardMaterial>>()
                .insert(MeshMaterial3d::<EarthMaterial>(Handle::clone(existing)));
            continue;
        }
        let mut base = base.clone();
        base.unlit = true;
        let m = meshes.get_mut(&mesh.0).unwrap();
        let colors: Vec<_> = (0..m.count_vertices())
            .map(|i| [1., 1., 1., region.map_or(1., |r| r.0[i])])
            .collect();
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        let parameters = Parameters {
            eye: Vec4::ZERO,
            sun: Vec4::ZERO,
            layer: Vec4::new(
                RADIUS as f32,
                haze.map_or(5200., |h| h.0),
                haze.map_or(0., |h| h.1),
                if haze.is_some() {
                    0.
                } else if cloud.is_some() {
                    1.
                } else {
                    2.
                },
            ),
        };
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d({
                let handle = extended.add(EarthMaterial {
                    base,
                    extension: EarthExtension { parameters, nordic: Some(nordic.clone()) },
                });
                shared.insert(material.0.id(), handle.clone());
                handle
            }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_conversion_retains_one_opaque_globe_with_shared_regional_detail() {
        let mut app = App::new();
        let mut meshes = Assets::<Mesh>::default();
        let globe = meshes.add(crate::world::globe_mesh(16, 8));
        let mut materials = Assets::<StandardMaterial>::default();
        let material = materials.add(StandardMaterial::default());
        let entity = app.world_mut().spawn((Surface, Mesh3d(globe.clone()), MeshMaterial3d(material))).id();
        app.insert_resource(meshes).insert_resource(materials)
            .insert_resource(Assets::<Image>::default())
            .insert_resource(Assets::<EarthMaterial>::default())
            .add_systems(Update, convert);
        app.update();
        let entity = app.world().entity(entity);
        assert_eq!(entity.get::<Mesh3d>().unwrap().0, globe);
        assert!(entity.get::<MeshMaterial3d<StandardMaterial>>().is_none());
        let handle = &entity.get::<MeshMaterial3d<EarthMaterial>>().unwrap().0;
        let material = app.world().resource::<Assets<EarthMaterial>>().get(handle).unwrap();
        assert_eq!(material.base.alpha_mode, AlphaMode::Opaque);
        assert!(material.base.unlit);
        assert!(material.extension.nordic.is_some());
        assert_eq!(material.extension.parameters.layer.w, 2.);
        assert_eq!(app.world_mut().query::<&Surface>().iter(app.world()).count(), 1);
        assert_eq!(app.world_mut().query::<&Region>().iter(app.world()).count(), 0);
    }
}
