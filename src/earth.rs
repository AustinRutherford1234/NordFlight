//! Continuous Earth appearance shared by flight and orbital cartography.
use crate::{FlightCamera, ViewState, physics::*, world};
use bevy::{mesh::VertexAttributeValues, prelude::*};
#[path = "earth_region.rs"]
pub(crate) mod region;
pub use region::setup as setup_region;
#[derive(Component)]
pub struct Surface;
#[derive(Component)]
pub struct LocalWater;
#[derive(Component)]
pub struct Haze(pub f32, pub f32);
#[derive(Component)]
pub struct CloudCover;

pub fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}
pub fn terrain_opacity(altitude: f64) -> f32 {
    1.0 - smoothstep(((altitude - 18000.0) / 42000.0) as f32)
}
pub fn sky_density(altitude: f64) -> f32 {
    1.0 - smoothstep(((altitude - 45000.0) / 50000.0) as f32)
}
/// Conservative zero-contribution culling for the existing shell shader. Its
/// haze alpha is zero below layer height; cloud alpha is zero below 2000 m.
/// Retain a two-metre margin for CPU/GPU f32 length rounding at Earth radius.
fn shell_visibility(eye: Vec3, haze: Option<&Haze>, cloud: bool) -> Visibility {
    let cutoff = haze.map(|h|h.0).or(cloud.then_some(2000.));
    if std::env::var_os("NORD_EARTH_DRAW_ZERO_ALPHA").is_none()
        && cutoff.is_some_and(|h|eye.length() < RADIUS as f32 + h - 2.) {
        Visibility::Hidden
    } else { Visibility::Inherited }
}
fn cloud_alpha(brightness: u8) -> u8 {
    ((brightness as f32 / 255.0).powf(1.25) * 220.0) as u8
}
pub fn readable_oceans(image: &mut Image) {
    // Lift dark water candidates, including near-black inland lakes.
    if image.texture_descriptor.format
        != bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb
    {
        return;
    }
    if let Some(data) = image.data.as_mut() {
        for pixel in data.chunks_exact_mut(4) {
            let (r, g, b) = (pixel[0] as u16, pixel[1] as u16, pixel[2] as u16);
            if (b >= r + 3 && b >= g + 2 && r < 48 && g < 64) || (r < 8 && g < 12 && b < 18) {
                pixel[0] = pixel[0].max(10);
                pixel[1] = pixel[1].max(28);
                pixel[2] = pixel[2].max(50);
            }
        }
    }
}

pub fn setup(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    for (height, strength) in [(8000.0, 0.38), (22000.0, 0.19), (50000.0, 0.065)] {
        let mut mesh = world::globe_mesh(512, 256);
        if let Some(VertexAttributeValues::Float32x3(p)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            for p in p {
                for a in p {
                    *a *= (RADIUS as f32 + height) / (RADIUS as f32 - 80.0);
                }
            }
        }
        c.spawn((
            Haze(height, strength),
            world::Globe,
            world::WorldObject {
                position: bevy::math::DVec3::ZERO,
                rotation: bevy::math::DQuat::IDENTITY,
            },
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::WHITE,
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::default(),
            Visibility::Hidden,
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
        ));
    }
    let mut mesh = world::globe_mesh(512, 256);
    if let Some(VertexAttributeValues::Float32x3(p)) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for point in p {
            let n = Vec3::from(*point).normalize();
            *point = (n * (RADIUS as f32 + 5200.0)).to_array();
        }
    }
    let mut cloud_image = Image::from_buffer(
        include_bytes!("../assets/earth-clouds.jpg"),
        bevy::image::ImageType::Extension("jpg"),
        bevy::image::CompressedImageFormats::NONE,
        true,
        bevy::image::ImageSampler::default(),
        bevy::asset::RenderAssetUsages::default(),
    )
    .unwrap();
    if let Some(data) = cloud_image.data.as_mut() {
        for p in data.chunks_exact_mut(4) {
            let alpha = cloud_alpha(p[0]);
            p.copy_from_slice(&[255, 255, 255, alpha]);
        }
    }
    cloud_image.sampler =
        bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
            address_mode_u: bevy::image::ImageAddressMode::Repeat,
            address_mode_v: bevy::image::ImageAddressMode::ClampToEdge,
            ..bevy::image::ImageSamplerDescriptor::linear()
        });
    c.spawn((
        CloudCover,
        world::Globe,
        world::WorldObject {
            position: bevy::math::DVec3::ZERO,
            rotation: bevy::math::DQuat::IDENTITY,
        },
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(images.add(cloud_image)),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })),
        Transform::default(),
        bevy::light::NotShadowCaster,
        bevy::light::NotShadowReceiver,
    ));
}
pub fn daylight(dot: f32) -> f32 {
    let t = ((dot + 0.10) / 0.45).clamp(0., 1.);
    0.16 + 0.84 * t * t * (3. - 2. * t)
}
pub fn update(
    f: Res<Flight>,
    e: Res<Environment>,
    v: Res<ViewState>,
    camera: Query<&Transform, With<FlightCamera>>,
    water: Query<&MeshMaterial3d<StandardMaterial>, With<LocalWater>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut gpu_materials: ResMut<Assets<crate::earth_gpu::EarthMaterial>>,
    mut gpu_layers: Query<(&MeshMaterial3d<crate::earth_gpu::EarthMaterial>, Option<&Haze>, Option<&CloudCover>, &mut Visibility), Without<MeshMaterial3d<StandardMaterial>>>,
    mut globes: Query<
        (
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            Option<&Haze>,
            Option<&CloudCover>,
            Option<&region::Region>,
            &mut Visibility,
        ),
        Or<(With<Surface>, With<Haze>, With<CloudCover>)>,
    >,
    mut last: Local<Option<(Vec3, Vec3, bool)>>,
) {
    let _measure = crate::performance::span(3);
    for handle in &water {
        let opacity = terrain_opacity(f.altitude());
        let alpha_mode = if opacity > 0.999 { AlphaMode::Opaque } else { AlphaMode::Blend };
        if mats.get(&handle.0).is_some_and(|m| m.base_color.alpha() != opacity || m.alpha_mode != alpha_mode) {
            let mat = mats.get_mut(&handle.0).unwrap();
            mat.base_color = mat.base_color.with_alpha(opacity);
            mat.alpha_mode = alpha_mode;
        }
    }
    let sun = e.sun_direction().as_vec3();
    let eye = camera
        .iter()
        .next()
        .map(|t| (world::render_origin(&f) + world::frame(&f) * t.translation.as_dvec3()).as_vec3())
        .unwrap_or(Vec3::Y);
    let cached = last.is_some_and(|(light, view, map)| map == v.map
        && light.distance_squared(sun) < 1e-7 && view.distance_squared(eye) < 400.0);
    // Use the eye actually stored in the shader when its inputs are cached.
    // Otherwise a small descent across the shell boundary could hide a layer
    // whose cached alpha is still nonzero. Camera/map changes remain current.
    let visibility_eye = if cached {last.unwrap().1} else {eye};
    for (_,haze,cloud,mut visibility) in &mut gpu_layers {
        if haze.is_some() || cloud.is_some() {
            visibility.set_if_neq(shell_visibility(visibility_eye,haze,cloud.is_some()));
        }
    }
    for (_,_,haze,cloud,_,mut visibility) in &mut globes {
        if haze.is_some() || cloud.is_some() {
            visibility.set_if_neq(shell_visibility(visibility_eye,haze,cloud.is_some()));
        }
    }
    let light_changed = last.is_none_or(|(light, _, _)| light.distance_squared(sun) >= 1e-7);
    let cloud_changed = eye.length() - (RADIUS as f32) < 16000.0;
    if cached { return; }
    let applied_sun = if light_changed { sun } else { last.unwrap().0 };
    *last = Some((applied_sun, eye, v.map));
    for (handle, haze, clouds, _) in &mut gpu_layers {
        if haze.is_none() && !light_changed && (clouds.is_none() || !cloud_changed) { continue; }
        if let Some(material) = gpu_materials.get_mut(&handle.0) {
            material.extension.parameters.eye = eye.extend(0.);
            material.extension.parameters.sun = sun.extend(0.);
        }
    }
    for (handle, material, haze, clouds, region, _) in &mut globes {
        if haze.is_none() && !light_changed && (clouds.is_none() || !cloud_changed) {
            continue;
        }
        if haze.is_none() { if let Some(mat) = mats.get_mut(&material.0) {
            if !mat.unlit {
                mat.unlit = true;
            }
        } }
        let Some(mesh) = mesh_assets.get_mut(&handle.0) else {
            continue;
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            continue;
        };
        let colors: Vec<[f32; 4]> = normals
            .iter()
            .enumerate()
            .map(|(index, n)| {
                let n = Vec3::from(*n).normalize();
                if let Some(haze) = haze {
                    let view = (eye - n * (RADIUS as f32 + haze.0)).normalize_or_zero();
                    let mu = n.dot(view).abs();
                    let rim = (1.0 - mu).powi(6) * smoothstep(mu / 0.16);
                    let outside = smoothstep((eye.length() - RADIUS as f32 - haze.0) / 35000.0);
                    let column = if haze.0 < 10000.0 {
                        0.035
                    } else if haze.0 < 30000.0 {
                        0.012
                    } else {
                        0.004
                    };
                    [
                        0.20,
                        0.50,
                        1.0,
                        (column + rim * haze.1) * outside * daylight(n.dot(sun)),
                    ]
                } else if clouds.is_some() {
                    let shade = daylight(n.dot(sun));
                    let near = smoothstep(((eye.length() - RADIUS as f32) - 2000.0) / 12000.0);
                    let distance = (eye - n * (RADIUS as f32 + 5200.0)).length();
                    [
                        shade * 0.94,
                        shade * 0.97,
                        shade,
                        near * smoothstep(distance / 8000.0),
                    ]
                } else {
                    let shade = daylight(n.dot(sun));
                    [shade, shade, shade, region.map_or(1.0, |r| r.0[index])]
                }
            })
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shell_visibility_uses_cached_shader_eye_then_refreshes_across_boundary() {
        use crate::earth_gpu::{EarthExtension,EarthMaterial,Parameters};
        let mut app=App::new();
        let f=Flight::runway();
        let origin=world::render_origin(&f);
        let inverse=world::frame(&f).conjugate();
        let camera_pose=|altitude:f64| Transform::from_translation(
            (inverse*(bevy::math::DVec3::Y*(RADIUS+altitude)-origin)).as_vec3());
        let mut materials=Assets::<EarthMaterial>::default();
        let handle=materials.add(EarthMaterial {
            base:StandardMaterial::default(),
            extension:EarthExtension {parameters:Parameters {
                eye:Vec4::ZERO,sun:Vec4::ZERO,layer:Vec4::new(RADIUS as f32,8000.,0.38,0.)
            },nordic:None}
        });
        app.insert_resource(f).insert_resource(Environment::default())
            .insert_resource(ViewState::default())
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<StandardMaterial>::default())
            .insert_resource(materials).add_systems(Update,update);
        let camera=app.world_mut().spawn((FlightCamera,camera_pose(8005.))).id();
        let shell=app.world_mut().spawn((Haze(8000.,0.38),MeshMaterial3d(handle.clone()),Visibility::Inherited)).id();
        app.update();
        let first_eye=app.world().resource::<Assets<EarthMaterial>>().get(&handle).unwrap().extension.parameters.eye;
        *app.world_mut().entity_mut(camera).get_mut::<Transform>().unwrap()=camera_pose(7995.);
        app.update();
        assert_eq!(app.world().entity(shell).get::<Visibility>(),Some(&Visibility::Inherited));
        assert_eq!(app.world().resource::<Assets<EarthMaterial>>().get(&handle).unwrap().extension.parameters.eye,first_eye);
        *app.world_mut().entity_mut(camera).get_mut::<Transform>().unwrap()=camera_pose(7970.);
        app.update();
        assert_eq!(app.world().entity(shell).get::<Visibility>(),Some(&Visibility::Hidden));
        *app.world_mut().entity_mut(camera).get_mut::<Transform>().unwrap()=camera_pose(7999.);
        app.update();
        assert_eq!(app.world().entity(shell).get::<Visibility>(),Some(&Visibility::Inherited));
    }
    #[test]
    fn shell_culling_reopens_before_any_nonzero_opacity() {
        for cutoff in [2000.,8000.,22000.,50000.] {
            let haze=Haze(cutoff,0.38);
            for altitude in [0.,cutoff-3.,cutoff-2.,cutoff,cutoff+1.,cutoff+3000.] {
                let eye=Vec3::Y*(RADIUS as f32+altitude);
                let visibility=shell_visibility(eye,Some(&haze),false);
                let alpha_scale=smoothstep((eye.length()-RADIUS as f32-cutoff)/35000.);
                if visibility==Visibility::Hidden {assert_eq!(alpha_scale,0.);}
                if altitude>=cutoff-2. {assert_eq!(visibility,Visibility::Inherited);}
            }
        }
        assert_eq!(shell_visibility(Vec3::Y*(RADIUS as f32+500.),None,true),Visibility::Hidden);
        assert_eq!(shell_visibility(Vec3::Y*(RADIUS as f32+2000.),None,true),Visibility::Inherited);
        assert_eq!(shell_visibility(Vec3::Y*RADIUS as f32,None,false),Visibility::Inherited);
    }
    #[test]
    fn atmosphere_and_terrain_transitions_are_continuous_and_reversible() {
        let mut previous_sky = 1.0;
        let mut previous_terrain = 1.0;
        for altitude in (0..=120000).step_by(100) {
            let sky = sky_density(altitude as f64);
            let terrain = terrain_opacity(altitude as f64);
            assert!((0.0..=1.0).contains(&sky) && (0.0..=1.0).contains(&terrain));
            assert!(sky <= previous_sky && terrain <= previous_terrain);
            assert!((sky - previous_sky).abs() < 0.004);
            assert!((terrain - previous_terrain).abs() < 0.004);
            previous_sky = sky;
            previous_terrain = terrain;
        }
        assert_eq!(sky_density(99000.0), sky_density(101000.0));
        assert_eq!(terrain_opacity(18000.0), 1.0);
        assert_eq!(terrain_opacity(60000.0), 0.0);
    }
    #[test]
    fn satellite_cloud_alpha_preserves_clear_sky_and_bounded_opacity() {
        assert_eq!(cloud_alpha(0), 0);
        assert_eq!(cloud_alpha(255), 220);
        for i in 0..255 {
            assert!(cloud_alpha(i) <= cloud_alpha(i + 1));
        }
    }
    #[test]
    fn map_daylight_is_bounded_smooth_and_keeps_night_readable() {
        let mut old = daylight(-1.);
        for i in 0..=200 {
            let x = -1. + i as f32 / 100.;
            let value = daylight(x);
            assert!((0.16..=1.00001).contains(&value));
            assert!(value >= old - 1e-6);
            assert!((value - old).abs() < 0.04);
            old = value;
        }
    }
}
