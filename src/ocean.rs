//! Seeded Gaussian sea spectrum. Geometry and normals stay on the GPU; CPU
//! contact queries evaluate the same bounded 24-mode field in double precision.
use crate::{
    FlightCamera,
    physics::{Flight, planet_radius},
    world,
};
use bevy::{
    asset::RenderAssetUsages,
    asset::{load_internal_asset, uuid_handle},
    light::{NotShadowCaster, NotShadowReceiver},
    math::{DQuat, DVec2, DVec3},
    mesh::Indices,
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{
        AsBindGroup, Extent3d, PrimitiveTopology, ShaderType, TextureDimension, TextureFormat,
    },
    shader::{Shader, ShaderRef},
};
mod shore;

const SHADER: Handle<Shader> = uuid_handle!("2a9ec418-a6c8-430c-8d24-58bff5863856");
const COUNT: usize = 24;
const TAU: f64 = std::f64::consts::TAU;
#[derive(Clone, Copy, Debug)]
struct Wave {
    k: DVec2,
    a: f64,
    b: f64,
    omega: f64,
}
struct Random(u64);
impl Random {
    fn uniform(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn gaussian(&mut self) -> (f64, f64) {
        let r = (-2. * self.uniform().max(1e-12).ln()).sqrt();
        let t = TAU * self.uniform();
        (r * t.cos(), r * t.sin())
    }
}
fn spectrum() -> &'static [Wave; COUNT] {
    static WAVES: std::sync::OnceLock<[Wave; COUNT]> = std::sync::OnceLock::new();
    WAVES.get_or_init(|| {
        let mut rng = Random(0xa8b31d4295e761c3);
        let mut waves = std::array::from_fn(|i| {
            let wavelength = 12. * (220f64 / 12.).powf(i as f64 / (COUNT - 1) as f64);
            let direction = 0.58 + (rng.uniform() - 0.5) * 1.65;
            let k = DVec2::new(direction.cos(), direction.sin()) * (TAU / wavelength);
            let (a, b) = rng.gaussian();
            // Broad wind sea plus swell; log-spaced bins, no short-wave energy
            // in geometry that the mesh cannot resolve. Not a calibrated forecast.
            let weight = (-((wavelength.ln() - 65f64.ln()) / 0.95).powi(2) * 0.5).exp();
            Wave {
                k,
                a: a * weight,
                b: b * weight,
                omega: ((if !crate::environment::real(){9.80665}else{crate::environment::active().preset.planet.surface_gravity()}) * k.length()).sqrt(),
            }
        });
        let variance = waves
            .iter()
            .map(|w| (w.a * w.a + w.b * w.b) * 0.5)
            .sum::<f64>();
        let scale = 0.45 / variance.sqrt(); // Hs ~= 4 sigma = 1.8 m offshore.
        for w in &mut waves {
            w.a *= scale;
            w.b *= scale;
        }
        waves
    })
}
pub fn enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("NORD_OCEAN_REFERENCE").is_none())
}
fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn strength(p: DVec2) -> f64 {
    if crate::environment::real(){return 0.65;}
    0.10 + 0.90
        * (1. - smooth(6500., 7000., (p.x - 20000.).abs()))
        * (1. - smooth(-8500., -8000., p.y))
}
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub height: f64,
    pub normal: DVec3,
    pub velocity: DVec3,
}
pub fn sample(p: DVec3, time: f64) -> Sample {
    if !enabled() {
        return Sample {
            height: if crate::environment::real(){crate::environment::active().raw(p).2.unwrap_or(0.)}else{0.},
            normal: p.normalize(),
            velocity: DVec3::ZERO,
        };
    }
    let x = DVec2::new(p.x, p.z);
    let weight = strength(x);
    let mut h = 0.;
    let mut slope = DVec2::ZERO;
    let mut speed = 0.;
    for w in spectrum() {
        let phase = (w.k.dot(x) - w.omega * time).rem_euclid(TAU);
        let (s, c) = phase.sin_cos();
        h += w.a * c + w.b * s;
        let derivative = -w.a * s + w.b * c;
        slope += w.k * derivative;
        speed -= w.omega * derivative;
    }
    // Include amplitude-envelope gradient so coast transition normals agree.
    let dx = (strength(x + DVec2::X) - strength(x - DVec2::X)) * 0.5;
    let dz = (strength(x + DVec2::Y) - strength(x - DVec2::Y)) * 0.5;
    let up = p.normalize();
    Sample {
        height: if crate::environment::real(){crate::environment::active().raw(p).2.unwrap_or(0.)}else{0.} + 0.12 + h * weight,
        normal: (up - DVec3::new(slope.x * weight + h * dx, 0., slope.y * weight + h * dz))
            .normalize(),
        velocity: up * (speed * weight),
    }
}
#[derive(Clone, Copy, Debug, ShaderType, PartialEq)]
struct Parameters {
    center: Vec4,
    water_grid: Vec4,
    water_coarse:Vec4,
    detail: Vec4,
    axis_x: Vec4,
    axis_z: Vec4,
    waves: [Vec4; COUNT],
    phases: [Vec4; COUNT],
}
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct Extension {
    #[uniform(100)]
    parameters: Parameters,
    #[texture(101)]
    #[sampler(102)]
    detail: Handle<Image>,
    #[texture(103,sample_type="float",filterable=false)]
    water:Handle<Image>,
    #[texture(104,sample_type="float",filterable=false)]
    coarse:Handle<Image>,
}
impl MaterialExtension for Extension {
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
type OceanMaterial = ExtendedMaterial<StandardMaterial, Extension>;
#[derive(Component)]
pub(crate) struct Ocean;
#[derive(Component)]
pub(crate) struct Lakes;
#[derive(Resource)]
pub(crate) struct WaterGrid {
    image:Handle<Image>,coarse:Handle<Image>,cell:(i32,i32),bounds:Vec4,far:Vec4,
    task:Option<bevy::tasks::Task<shore::BuiltWater>>,
    lake_entity:Entity, lake_mesh:Handle<Mesh>, lake_anchor:DVec3,
    pending:Option<WaterPublication>,material:Handle<OceanMaterial>,
}
struct WaterPublication {
    water:Handle<Image>,coarse:Handle<Image>,mesh:Handle<Mesh>,material:Handle<OceanMaterial>,
    bounds:Vec4,far:Vec4,anchor:DVec3,
}
#[derive(Resource,Clone,Default)]
pub(crate) struct WaterReady(crate::terrain_ready::TerrainReady);
fn water_ready(state:Res<WaterReady>,
    meshes:Res<bevy::render::render_asset::RenderAssets<bevy::render::mesh::RenderMesh>>,
    images:Res<bevy::render::render_asset::RenderAssets<bevy::render::texture::GpuImage>>,
    materials:Res<bevy::render::erased_render_asset::ErasedRenderAssets<bevy::pbr::PreparedMaterial>>) {
    state.0.poll_all(|id|meshes.get(id).is_some(),|id|images.get(id).is_some(),|id|materials.get(id).is_some());
}
fn water_at_xz(environment:&crate::environment::EnvironmentSnapshot,x:f64,z:f64)->Option<f64> {
    let h=environment.height_xz(x,z);
    environment.raw(crate::environment::point(x,z,h,environment.preset.planet.radius)).2
}
fn water_image(x:f64,z:f64,n:usize,step:f64)->(Image,Vec4) {
    let west=(x/1024.).floor()*1024.-n as f64*step*0.5;
    let south=(z/1024.).floor()*1024.-n as f64*step*0.5;
    let mut bytes=Vec::with_capacity(n*n*8);
    for j in 0..n {for i in 0..n {
        let x=west+i as f64*step;let z=south+j as f64*step;
        // Terrain vertices retain Cartesian x/z when lifted. Sample the mask at
        // that same lifted geographic coordinate; sea-level sampling shifts
        // mountain lake shorelines by tens of metres relative to the ground.
        let water=water_at_xz(crate::environment::active(),x,z);
        bytes.extend((water.unwrap_or(0.) as f32).to_le_bytes());
        bytes.extend((if water.is_some(){1f32}else{0f32}).to_le_bytes());
    }}
    (Image::new(Extent3d{width:n as u32,height:n as u32,depth_or_array_layers:1},TextureDimension::D2,bytes,TextureFormat::Rg32Float,RenderAssetUsages::RENDER_WORLD),Vec4::new(west as f32,south as f32,step as f32,1.))
}

pub struct OceanPlugin;
impl Plugin for OceanPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER, "ocean.wgsl", Shader::from_wgsl);
        app.add_plugins(MaterialPlugin::<OceanMaterial>::default())
            .add_systems(PostStartup, setup);
        let ready=WaterReady::default();app.insert_resource(ready.clone());
        if let Some(render)=app.get_sub_app_mut(bevy::render::RenderApp) {
            render.insert_resource(ready).add_systems(bevy::render::Render,water_ready
                .in_set(bevy::render::RenderSystems::PrepareAssets)
                .after(bevy::render::render_asset::prepare_assets::<bevy::render::mesh::RenderMesh>));
        }
    }
}
fn parameters(center: DVec3, time: f64, quality: bool) -> Parameters {
    Parameters {
        water_grid:Vec4::ZERO,
        water_coarse:Vec4::ZERO,
        axis_x: Vec4::X,
        axis_z: Vec4::Z,
        center: Vec4::new(
            center.x as f32,
            center.z as f32,
            center.y as f32,
            planet_radius() as f32,
        ),
        detail: Vec4::new(
            (time * 0.13).rem_euclid(256.) as f32,
            (time * 0.07).rem_euclid(256.) as f32,
            if quality { 1. } else { 0. },
            0.,
        ),
        waves: std::array::from_fn(|i| {
            let w = spectrum()[i];
            Vec4::new(w.k.x as f32, w.k.y as f32, w.a as f32, w.b as f32)
        }),
        phases: std::array::from_fn(|i| {
            Vec4::new(
                (spectrum()[i].k.dot(DVec2::new(center.x, center.z)) - spectrum()[i].omega * time)
                    .rem_euclid(TAU) as f32,
                0.,
                0.,
                0.,
            )
        }),
    }
}
fn mesh() -> Mesh {
    // Concentric rings share every boundary. Small near-camera cells grow
    // smoothly offshore, without tessellation, LOD cracks or mesh replacement.
    let n = 256usize;
    let mut points = vec![[0., 0., 0.]];
    let mut uv = vec![[0., 0.]];
    let mut indices = Vec::new();
    let mut radius = 4f32;
    let mut rings = 0usize;
    while radius < 90000. {
        for j in 0..n {
            let a = j as f32 * std::f32::consts::TAU / n as f32;
            let x = a.cos() * radius;
            let z = a.sin() * radius;
            points.push([x, 0., z]);
            uv.push([x, z]);
        }
        radius *= 1.055;
        rings += 1;
    }
    for j in 0..n {
        indices.extend([0, 1 + ((j + 1) % n) as u32, 1 + j as u32]);
    }
    for ring in 1..rings {
        let a = 1 + (ring - 1) * n;
        let b = 1 + ring * n;
        for j in 0..n {
            let k = (j + 1) % n;
            indices.extend([
                (a + j) as u32,
                (a + k) as u32,
                (b + j) as u32,
                (a + k) as u32,
                (b + k) as u32,
                (b + j) as u32,
            ]);
        }
    }
    let normals = vec![[0., 1., 0.]; points.len()];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1f32; 4]; points.len()]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, points);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, vec![[0f32;2];uv.len()]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
fn detail_image() -> Image {
    // Periodic slope field with a mip chain: generated once, never uploaded per
    // frame. Independent rotated sampling in the shader hides the base repeat.
    let size = 512usize;
    let mut rng = Random(0x13e97ab4df620c58);
    let terms: Vec<_> = (0..64)
        .map(|_| {
            let x = (rng.uniform() * 129.).floor() as i32 - 64;
            let y = (rng.uniform() * 129.).floor() as i32 - 64;
            let (a, b) = rng.gaussian();
            {
                let norm = ((x * x + y * y) as f64).sqrt().max(1.);
                (x, y, a * 0.11 / norm, b * 0.11 / norm)
            }
        })
        .collect();
    let trig: Vec<_> = terms
        .iter()
        .map(|&(kx, ky, _, _)| {
            let x: Vec<_> = (0..size)
                .map(|i| (TAU * kx as f64 * i as f64 / size as f64).sin_cos())
                .collect();
            let y: Vec<_> = (0..size)
                .map(|i| (TAU * ky as f64 * i as f64 / size as f64).sin_cos())
                .collect();
            (x, y)
        })
        .collect();
    let mut pixels = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let mut slope = DVec2::ZERO;
            for (&(kx, ky, a, b), (xs, ys)) in terms.iter().zip(&trig) {
                let (sx, cx) = xs[x];
                let (sy, cy) = ys[y];
                let s = sx * cy + cx * sy;
                let c = cx * cy - sx * sy;
                slope += DVec2::new(kx as f64, ky as f64) * (-a * s + b * c) * 0.12;
            }
            pixels.extend([
                ((slope.x.clamp(-1., 1.) * 0.5 + 0.5) * 255.) as u8,
                ((slope.y.clamp(-1., 1.) * 0.5 + 0.5) * 255.) as u8,
                128,
                255,
            ]);
        }
    }
    let mut data = pixels.clone();
    let mut width = size;
    while width > 1 {
        let next = width / 2;
        let mut down = vec![0u8; next * next * 4];
        for y in 0..next {
            for x in 0..next {
                for c in 0..4 {
                    let sum = (0..2)
                        .flat_map(|dy| (0..2).map(move |dx| (dy, dx)))
                        .map(|(dy, dx)| pixels[((y * 2 + dy) * width + x * 2 + dx) * 4 + c] as u16)
                        .sum::<u16>();
                    down[(y * next + x) * 4 + c] = (sum / 4) as u8;
                }
            }
        }
        data.extend_from_slice(&down);
        pixels = down;
        width = next;
    }
    let mut image = Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0; size * size * 4],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(data);
    image.texture_descriptor.mip_level_count = 10;
    use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}
fn setup(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OceanMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    if !enabled() {
        return;
    }
    let built=shore::build(0.,0.);
    let water=images.add(built.near);let coarse=images.add(built.coarse);
    let lake_mesh=meshes.add(built.lakes);
    let material = materials.add(OceanMaterial {
        base: StandardMaterial {
            base_color: Color::srgb(0.018, 0.095, 0.125),
            metallic: 0.,
            perceptual_roughness: 0.24,
            reflectance: 0.35,
            double_sided: true,
            cull_mode: None,
            ..default()
        },
        extension: Extension {
            parameters: parameters(DVec3::Y * planet_radius(), 0., false),
            detail: images.add(detail_image()),
            water:water.clone(),coarse:coarse.clone(),
        },
    });
    let lake_entity=c.spawn((Lakes,Mesh3d(lake_mesh.clone()),MeshMaterial3d(material.clone()),
        Transform::default(),Visibility::Hidden,NotShadowCaster,NotShadowReceiver,
        bevy::camera::visibility::NoFrustumCulling)).id();
    c.insert_resource(WaterGrid{image:water,coarse,cell:(0,0),bounds:built.bounds,far:built.far,
        task:None,lake_entity,lake_mesh,lake_anchor:built.anchor,pending:None,material:material.clone()});
    c.spawn((
        Ocean,
        Mesh3d(meshes.add(mesh())),
        MeshMaterial3d(material),
        Transform::default(),
        Visibility::Hidden,
        NotShadowCaster,
        NotShadowReceiver,
        bevy::camera::visibility::NoFrustumCulling,
    ));
}
pub fn update(
    f: Res<Flight>,
    v: Res<crate::ViewState>,
    camera: Query<&Transform, (With<FlightCamera>, Without<Ocean>,Without<Lakes>)>,
    mut surfaces: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<OceanMaterial>,
        ),
        (With<Ocean>,Without<Lakes>),
    >,
    mut materials: ResMut<Assets<OceanMaterial>>,
    mut images:ResMut<Assets<Image>>,
    grid:Option<ResMut<WaterGrid>>,
    mut meshes:ResMut<Assets<Mesh>>,
    ready:Res<WaterReady>,
    mut lakes:Query<(&mut Transform,&mut Visibility,&mut Mesh3d,&mut MeshMaterial3d<OceanMaterial>),(With<Lakes>,Without<Ocean>,Without<FlightCamera>)>,
) {
    let Some(mut grid)=grid else{return;};
    for (mut transform, mut visible, mut handle) in &mut surfaces {
        let opacity = crate::earth::terrain_opacity(f.altitude());
        let show = opacity > 0.001 && !v.map && f.position.y > planet_radius() * 0.5
            && !(f.vehicle==crate::physics::Vehicle::Spacecraft&&f.space.body==crate::space::Body::Moon);
        visible.set_if_neq(if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if !show {
            if let Ok((_,mut visibility,_,_))=lakes.get_mut(grid.lake_entity){*visibility=Visibility::Hidden;}
            continue;
        }
        let eye = camera
            .iter()
            .next()
            .map(|t| world::render_origin(&f) + world::frame(&f) * t.translation.as_dvec3())
            .unwrap_or(f.position);
        let x = (eye.x / 4.).round() * 4.;
        let z = (eye.z / 4.).round() * 4.;
        let center = crate::terrain::point(x, z, 0.);
        transform.set_if_neq(Transform {
            translation: world::local(center, &f),
            rotation: world::local_rot(DQuat::IDENTITY, &f),
            ..default()
        });
        if crate::environment::real() {
            use bevy::tasks::{block_on,poll_once,AsyncComputeTaskPool};
            if let Some(task)=grid.task.as_mut() {
                if let Some(built)=block_on(poll_once(task)){
                    let water=images.add(built.near);let coarse=images.add(built.coarse);
                    let mesh=meshes.add(built.lakes);
                    if let Some(source)=materials.get(&handle.0).cloned(){
                        let mut next=source;next.extension.water=water.clone();next.extension.coarse=coarse.clone();
                        next.extension.parameters.water_grid=built.bounds;next.extension.parameters.water_coarse=built.far;
                        let material=materials.add(next);
                        ready.0.request(vec![mesh.id()]);
                        ready.0.dependencies(vec![water.id(),coarse.id()],vec![material.id().untyped()]);
                        grid.pending=Some(WaterPublication{water,coarse,mesh,material,bounds:built.bounds,far:built.far,anchor:built.anchor});
                    }
                    grid.task=None;
                }
            }
            let cell=((x/4096.).floor()as i32,(z/4096.).floor()as i32);
            if grid.pending.is_some()&&ready.0.ready(){
                let next=grid.pending.take().unwrap();
                grid.image=next.water;grid.coarse=next.coarse;grid.lake_mesh=next.mesh;
                grid.bounds=next.bounds;grid.far=next.far;grid.lake_anchor=next.anchor;
                handle.0=next.material;grid.material=handle.0.clone();ready.0.clear();
            }
            if grid.task.is_none()&&grid.pending.is_none()&&cell!=grid.cell {
                grid.cell=cell;
                grid.task=Some(AsyncComputeTaskPool::get().spawn(async move{
                    shore::build((x/4096.).floor()*4096.,(z/4096.).floor()*4096.)
                }));
            }
        }
        if let Ok((mut t,mut visibility,mut mesh,mut material))=lakes.get_mut(grid.lake_entity) {
            *t=Transform::from_translation(world::local(grid.lake_anchor,&f))
                .with_rotation(world::local_rot(DQuat::IDENTITY,&f));
            mesh.0=grid.lake_mesh.clone();material.0=handle.0.clone();visibility.set_if_neq(*visible);
        }
        let mut desired = parameters(center, f.elapsed, v.high_quality);
        desired.detail.w=crate::environment::active().preset.planet.sea_level as f32;
        if crate::environment::real(){desired.water_grid=grid.bounds;desired.water_coarse=grid.far;}
        desired.axis_x = (world::frame(&f).conjugate() * DVec3::X)
            .as_vec3()
            .extend(0.);
        desired.axis_z = (world::frame(&f).conjugate() * DVec3::Z)
            .as_vec3()
            .extend(0.);
        let alpha = if opacity > 0.999 {
            AlphaMode::Opaque
        } else {
            AlphaMode::Blend
        };
        let changed = materials.get(&handle.0).is_some_and(|m| {
            m.extension.parameters != desired
                || m.base.alpha_mode != alpha
                || m.base.base_color.alpha() != opacity
        });
        if changed {
            if let Some(m) = materials.get_mut(&handle.0) {
                m.extension.parameters = desired;
                m.base.alpha_mode = alpha;
                m.base.base_color = m.base.base_color.with_alpha(opacity);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mountain_water_mask_samples_the_same_lifted_surface_as_terrain() {
        let mut environment=crate::environment::EnvironmentSnapshot::legacy();
        environment.preset.legacy=false;
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/environment/packs");
        environment.data=crate::environment::pack::TerrainData::load(&root,&["norway-mountains".into()],384*1024*1024).unwrap();
        let mut regressions=0;
        for j in 0..100 {for i in 0..100 {
            let(x,z)=(289000.+i as f64*96.,297000.+j as f64*96.);
            let h=environment.height_xz(x,z);
            let expected=environment.raw(crate::environment::point(x,z,h,environment.preset.planet.radius)).2;
            let old=environment.raw(crate::environment::point(x,z,0.,environment.preset.planet.radius)).2;
            if old!=expected {assert_eq!(water_at_xz(&environment,x,z),expected);regressions+=1;}
        }}
        assert!(regressions>0,"fixture must exercise the shifted mountain shoreline");
    }
    #[test]
    fn gaussian_spectrum_is_bounded_and_has_requested_variance() {
        let variance = spectrum()
            .iter()
            .map(|w| (w.a * w.a + w.b * w.b) * 0.5)
            .sum::<f64>();
        assert!((4. * variance.sqrt() - 1.8).abs() < 1e-12);
        assert!(
            spectrum()
                .iter()
                .map(|w| w.a.abs() + w.b.abs())
                .sum::<f64>()
                < 5.
        );
    }
    #[test]
    fn derivatives_match_finite_difference() {
        let p = crate::terrain::point(20000., -16000., 0.);
        let t = 37.;
        let a = sample(p, t);
        let d = 0.001;
        let dx =
            (sample(p + DVec3::X * d, t).height - sample(p - DVec3::X * d, t).height) / (2. * d);
        let dz =
            (sample(p + DVec3::Z * d, t).height - sample(p - DVec3::Z * d, t).height) / (2. * d);
        let n = (p.normalize() - DVec3::new(dx, 0., dz)).normalize();
        assert!(n.distance(a.normal) < 1e-8);
        let speed = (sample(p, t + d).height - sample(p, t - d).height) / (2. * d);
        assert!((speed - a.velocity.dot(p.normalize())).abs() < 1e-6);
    }
    #[test]
    fn gpu_phase_rebasing_preserves_world_wave_height() {
        let x = DVec2::new(20480.25, -17003.4);
        for center in [DVec2::new(20480., -17004.), DVec2::new(20512., -17036.)] {
            let p = parameters(crate::terrain::point(center.x, center.y, 0.), 123.4, false);
            let mut h = 0f32;
            for i in 0..COUNT {
                let w = p.waves[i];
                let phase =
                    w.x * (x.x - center.x) as f32 + w.y * (x.y - center.y) as f32 + p.phases[i].x;
                h += w.z * phase.cos() + w.w * phase.sin();
            }
            assert!(
                (h as f64 + 0.12 - sample(crate::terrain::point(x.x, x.y, 0.), 123.4).height).abs()
                    < 2e-6
            );
        }
    }
    #[test]
    fn continuous_mesh_has_bounded_geometry_and_mipped_detail() {
        let m = mesh();
        assert!(m.count_vertices() < 60000);
        let image = detail_image();
        assert_eq!(image.texture_descriptor.mip_level_count, 10);
        assert_eq!(
            image.data.as_ref().unwrap().len(),
            (0..10)
                .map(|i| ((512 >> i) * (512 >> i) * 4) as usize)
                .sum::<usize>()
        );
    }
}

pub(crate) fn loading_resources(f:&Flight,grid:Option<&WaterGrid>)->Option<crate::loading::Resources>{
    let mut result=crate::loading::Resources::default();
    if !enabled()||!crate::environment::real()||f.altitude()>100000.||f.position.y<planet_radius()*0.5{return Some(result);}
    let grid=grid?;
    let center=Vec2::new(grid.bounds.x+384.*32.,grid.bounds.y+384.*32.);
    if (center.x as f64-f.position.x).abs()>6000.||(center.y as f64-f.position.z).abs()>6000.
        ||grid.task.is_some()||grid.pending.is_some(){return None;}
    result.meshes.push(grid.lake_mesh.id());
    result.images.extend([grid.image.id(),grid.coarse.id()]);result.materials.push(grid.material.id().untyped());
    Some(result)
}
