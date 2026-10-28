//! Visual F/A-18F rig. The existing flight model remains authoritative.
use super::*;
#[path="hornet_gear.rs"]
mod gear_motion;

use bevy::{pbr::{ExtendedMaterial, MaterialExtension},render::render_resource::AsBindGroup,shader::ShaderRef};
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct HeatExtension {}
impl MaterialExtension for HeatExtension {
    fn fragment_shader()->ShaderRef {"shaders/hornet_heat.wgsl".into()}
    fn enable_prepass()->bool {false}
    fn enable_shadows()->bool {false}
}
type HeatMaterial=ExtendedMaterial<StandardMaterial,HeatExtension>;
pub struct HeatPlugin;
impl Plugin for HeatPlugin {
    fn build(&self,app:&mut App) {app.add_plugins(MaterialPlugin::<HeatMaterial>::default());}
}

#[derive(Component)]
pub(crate) struct ImportedHornet;
#[derive(Component)]
pub(crate) struct HornetExhaust(usize,usize);
#[derive(Component)]
pub(crate) struct HornetEngineGlow(usize);
#[derive(Component)]
pub(crate) struct HornetHeatHaze(usize);

pub(super) fn setup(c: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>, server: &AssetServer, root: Entity) {
    let model = c.spawn((
        ImportedHornet,
        SceneRoot(server.load(GltfAssetLabel::Scene(0).from_asset("aircraft/fa18f/rigged.glb"))),
        // Match the existing landing model's wheel height without retuning it.
        Transform::from_xyz(0., 0.35, 2.56)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::splat(0.1)),
        Visibility::Inherited,
    )).id();
    c.entity(root).add_child(model);

    let cabin = c.spawn((CockpitRoot(Aircraft::Hornet), Transform::from_translation(Aircraft::Hornet.cabin_offset()), Visibility::Inherited)).id();
    c.entity(root).add_child(cabin);
    crate::cockpit::setup_for(c, meshes, mats, images, cabin, Vehicle::Gripen, Aircraft::Hornet);
    crate::cockpit::fleet_display(c, meshes, mats, images, cabin, Aircraft::Hornet);

    let metal = material(mats, Color::srgb(0.58,0.62,0.65), 0.7);
    let spec = Aircraft::Hornet.spec();
    for i in 0..(spec.ammo[0]+spec.ammo[1]) as usize {
        let mesh = if i >= spec.ammo[0] as usize { crate::combat::radar_visual::body(false,true) } else { crate::combat::missile_mesh() };
        let e = part(c,meshes,root,mesh,metal.clone(),Transform::from_translation(super::super::station(Aircraft::Hornet,i).as_vec3()));
        c.entity(e).insert(Store(Aircraft::Hornet,i));
        let at=super::super::station(Aircraft::Hornet,i).as_vec3();
        // The rail saddle meets the missile body, with the pylon above it.
        let rail=box_part(c,meshes,root,metal.clone(),at+Vec3::Y*0.14,Vec3::new(0.10,0.12,1.35));
        c.entity(rail).insert(Name::new(format!("HornetLaunchRail{i}")));

    }
    let beacon=c.spawn((PointLight{color:Color::srgb(1.,0.03,0.02),intensity:0.,range:5.,shadows_enabled:false,..default()},
        Transform::from_xyz(0.,0.8,0.),Lamp(Aircraft::Hornet,1))).id();
    c.entity(root).add_child(beacon);
    // Measured aft nozzle rings in the imported asset: source centers
    // (+/-4.75 lateral, 1.30 up), aft lip -56.42. Apply the model's
    // 0.1 scale, +90deg yaw and (0, .35, 2.56) translation.
    let envelope=meshes.add(exhaust_envelope());
    let shock=meshes.add(exhaust_shock_cell());
    let face=meshes.add(engine_glow_disk());
    for (i, x) in [-0.475,0.475].into_iter().enumerate() {
        let nozzle=c.spawn((Nozzle(Aircraft::Hornet,i),Transform::from_xyz(x,0.48,8.202),Visibility::Inherited)).id();
        c.entity(root).add_child(nozzle);
        let light=c.spawn((HornetEngineGlow(i),PointLight {color:Color::srgb(1.,0.2,0.03),intensity:0.,range:1.6,shadows_enabled:false,..default()},Transform::from_xyz(0.,0.,-0.04))).id();
        c.entity(nozzle).add_child(light);
        let haze_material=mats.add(StandardMaterial {
            base_color:Color::WHITE,specular_transmission:1.,ior:1.018,
            thickness:0.24,perceptual_roughness:0.02,reflectance:0.,
            cull_mode:Some(bevy::render::render_resource::Face::Back),..default()
        });
        let haze=part(c,meshes,nozzle,heat_haze_mesh(0.,i),haze_material,Transform::default());
        c.entity(haze).insert((HornetHeatHaze(i),bevy::light::NotShadowCaster));
        c.queue(move |world:&mut World| {
            let handle=world.resource_mut::<Assets<HeatMaterial>>().add(HeatMaterial {
                base:StandardMaterial{specular_transmission:1.,alpha_mode:AlphaMode::Blend,reflectance:0.,..default()},extension:HeatExtension{}
            });
            world.entity_mut(haze).remove::<MeshMaterial3d<StandardMaterial>>().insert(MeshMaterial3d(handle));
        });
        for layer in 0..14 {
            let glow=mats.add(StandardMaterial{base_color:Color::srgba(0.18,0.35,1.,0.012),emissive:LinearRgba::new(3.,4.,15.,1.),alpha_mode:AlphaMode::Add,unlit:true,cull_mode:None,..default()});
            let mesh=if layer==13 {face.clone()} else if layer>=7 {shock.clone()} else {envelope.clone()};
            let e=c.spawn((Mesh3d(mesh),MeshMaterial3d(glow),Transform::default())).id();
            c.entity(nozzle).add_child(e);
            c.entity(e).insert((HornetExhaust(i,layer),bevy::light::NotShadowCaster));
        }
    }
}

/// A colorless refractive volume. Traveling normal ripples bend the opaque
/// background; no smoke color or additive glow is introduced.
fn heat_haze_mesh(time:f64,engine:usize)->Mesh {
    let mut positions=Vec::new();let mut normals=Vec::new();let mut colors=Vec::new();let mut indices=Vec::new();
    let phase=(time*9.0 % (std::f64::consts::TAU*100.)) as f32+engine as f32*2.4;
    for ring in 0..=32 {
        let u=ring as f32/32.;
        let fade=(u*8.).min(1.)*(1.-u).powf(1.5);
        let radius=(0.26+u*0.36)*(1.-u).powf(0.35);
        for side in 0..=32 {
            let a=side as f32/32.*std::f32::consts::TAU;
            let wave=(u*43.-phase+a*3.).sin()*0.55+(u*71.-phase*1.7-a*5.).sin()*0.25;
            let n=Vec3::new(a.cos()+wave*fade*0.7,a.sin()+wave*fade*0.6,wave*fade*1.5).normalize();
            positions.push([radius*a.cos(),radius*a.sin()+u*u*0.10,u*4.8]);
            normals.push(n.to_array());
            colors.push([1.,1.,1.,fade]);
            if ring<32 && side<32 {let i=ring*33+side;indices.extend_from_slice(&[i,i+1,i+33,i+1,i+34,i+33]);}
        }
    }
    Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList,bevy::asset::RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION,positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL,normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR,colors)
        .with_inserted_indices(bevy::mesh::Indices::U32(indices))
}

pub fn heat_haze(f:Res<Flight>,mut meshes:ResMut<Assets<Mesh>>,mut haze:Query<(&HornetHeatHaze,&Mesh3d,&mut Visibility,&mut Transform)>) {
    for (engine,mesh,mut visible,mut t) in &mut haze {
        let active=f.aircraft==Aircraft::Hornet && f.vehicle==Vehicle::Gripen && !f.crashed && f.engine && f.thrust>1. && f.systems.engine_on[engine.0] && f.systems.engines[engine.0]>0.1;
        *visible=if active {Visibility::Inherited}else{Visibility::Hidden};
        if !active {continue;}
        let power=f.spool.clamp(0.,1.) as f32;
        t.scale=Vec3::new(1.,1.,0.4+power*0.6);
        if let Some(m)=meshes.get_mut(&mesh.0) {*m=heat_haze_mesh(f.elapsed,engine.0);}
    }
}

/// Recessed radiant engine face: a radial falloff keeps the nozzle lip crisp.
fn engine_glow_disk()->Mesh {
    let mut positions=Vec::new();let mut colors=Vec::new();let mut normals=Vec::new();let mut indices=Vec::new();
    for ring in 0..=12 {
        let r=ring as f32/12.;
        for side in 0..=48 {
            let a=side as f32/48.*std::f32::consts::TAU;
            positions.push([r*a.cos(),r*a.sin(),-0.08*(1.-r*r)]);normals.push([0.,0.,1.]);
            colors.push([1.,1.,1.,((r-0.35)/0.4).clamp(0.,1.)*(1.-r).powf(0.45)]);
            if ring<12 && side<48 {let i=ring*49+side;indices.extend_from_slice(&[i,i+1,i+49,i+1,i+50,i+49]);}
        }
    }
    Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList,bevy::asset::RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION,positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL,normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR,colors)
        .with_inserted_indices(bevy::mesh::Indices::U32(indices))
}

fn exhaust_envelope()->Mesh {
    let mut positions=Vec::new(); let mut normals=Vec::new(); let mut colors=Vec::new(); let mut indices=Vec::new();
    for ring in 0..=24 {
        let u=ring as f32/24.;
        let radius=(1.-u).powf(0.65)*(0.82+0.18*(u*std::f32::consts::PI).sin())*(0.98+0.02*(u*std::f32::consts::TAU*5.).cos());
        let fade=(0.35+0.65*(u*12.).min(1.))*(1.-u).powf(1.4);
        for side in 0..=32 {
            let angle=side as f32/32.*std::f32::consts::TAU;
            let r=radius*(1.+0.025*(angle*5.+u*28.).sin());
            positions.push([r*angle.cos(),r*angle.sin(),u]);
            normals.push([angle.cos(),angle.sin(),0.]);
            colors.push([1.,1.,1.,fade]);
            if ring<24 && side<32 {let a=ring*33+side; indices.extend_from_slice(&[a,a+1,a+33,a+1,a+34,a+33]);}
        }
    }
    Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList,bevy::asset::RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION,positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL,normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR,colors)
        .with_inserted_indices(bevy::mesh::Indices::U32(indices))
}

// Soft double cones form compression cells without opaque spherical outlines.
fn exhaust_shock_cell()->Mesh {
    let mut positions=Vec::new(); let mut normals=Vec::new(); let mut colors=Vec::new(); let mut indices=Vec::new();
    for ring in 0..=16 {
        let u=ring as f32/16.;
        let radius=1.-(u*2.-1.).abs();
        for side in 0..=24 {
            let a=side as f32/24.*std::f32::consts::TAU;
            positions.push([radius*a.cos(),radius*a.sin(),u*2.-1.]);
            normals.push([a.cos(),a.sin(),0.]);
            colors.push([1.,1.,1.,radius.powf(2.2)]);
            if ring<16 && side<24 {let i=ring*25+side; indices.extend_from_slice(&[i,i+1,i+25,i+1,i+26,i+25]);}
        }
    }
    Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList,bevy::asset::RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION,positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL,normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR,colors)
        .with_inserted_indices(bevy::mesh::Indices::U32(indices))
}

/// Warm afterburner plume and a subtle recessed nozzle glow at dry power.
pub fn exhaust(f:Res<Flight>,mut materials:ResMut<Assets<StandardMaterial>>,mut jets:Query<(&HornetExhaust,&mut Transform,&mut Visibility,&MeshMaterial3d<StandardMaterial>)>,mut lights:Query<(&HornetEngineGlow,&mut PointLight)>) {
 for (jet,mut t,mut visible,handle) in &mut jets {
  let active=f.aircraft==Aircraft::Hornet && f.vehicle==Vehicle::Gripen && !f.crashed && f.engine && f.systems.engine_on[jet.0] && f.systems.engines[jet.0]>0.1 && f.thrust>1.;
  let power=f.spool.clamp(0.,1.) as f32;
  let wet=((power-0.88)/0.12).clamp(0.,1.);
  let cell=(7..13).contains(&jet.1);
  let internal=jet.1==13;
  *visible=if active && (internal || wet>0.02){Visibility::Inherited}else{Visibility::Hidden};
  if !active {continue;}
  let motion=crate::plume_motion::sample(f.elapsed,(jet.0*37+jet.1) as u32);
  let length=(0.35+power*0.80+wet*4.10)*motion.length;
  let (radius,extent,z,color,alpha)=if internal {
   (0.29,1.,-0.075,LinearRgba::new(2.0+wet*6.9,0.025+wet*4.,0.008+wet*2.5,1.),0.055+power*0.045+wet*0.11)
  }else if cell {
   let k=(jet.1-7) as f32;
   (0.16*(1.-k*0.085),0.40+wet*0.14,0.55+k*0.60,LinearRgba::new(8.,4.0,2.0,1.),wet*0.045*(1.-k*0.11))
  }else{
   let k=jet.1 as f32;
   let r=if jet.1<2 {0.10+k*0.025}else{0.18+k*0.022};
   let color=if jet.1<2 {LinearRgba::new(1.5+wet*6.,0.4+wet*3.2,0.1+wet*3.5,1.)}else{LinearRgba::new(0.2+wet*3.0,0.05+wet*0.65,0.02+wet*2.8,1.)};
   (r,length*(if jet.1<2 {0.52}else{1.-k*0.045}),0.,color,if jet.1<2 {0.035+power*0.035+wet*0.045} else {0.012+power*0.008+wet*0.025})
  };
  t.translation=Vec3::Z*(z-0.025);
  t.rotation=motion.bend;
  t.scale=Vec3::new(radius*motion.width,radius*motion.width,if cell {extent*0.5}else{extent});
  if let Some(m)=materials.get_mut(&handle.0) {
   // Unlit additive surfaces use base color for their visible radiance.
   // Keep the dry core visible rather than relying on emissive bloom.
   let core_gain=if !cell && !internal && jet.1<2 {1.+(1.-wet)*4.+wet*0.35}else{1.};
   m.base_color=Color::linear_rgba(color.red*0.15*core_gain*motion.radiance,color.green*0.15*core_gain*motion.radiance,color.blue*0.15*core_gain*motion.radiance,alpha*4.);
   m.emissive=color*(alpha*(0.7+power*0.6+wet*1.2)*0.22*motion.radiance*if !cell && !internal && jet.1<2 {1.+(1.-wet)*5.}else{1.});
  }
 }
 for (engine,mut light) in &mut lights {
  let active=f.aircraft==Aircraft::Hornet && f.vehicle==Vehicle::Gripen && !f.crashed && f.engine && f.systems.engine_on[engine.0] && f.systems.engines[engine.0]>0.1 && f.thrust>1.;
  let power=f.spool.clamp(0.,1.) as f32;let wet=((power-0.88)/0.12).clamp(0.,1.);
  light.intensity=if active {(2.+power*5.+wet*350.)*crate::plume_motion::sample(f.elapsed,engine.0 as u32).radiance}else{0.};
  light.color=Color::srgb(1.,0.035+wet*0.30,0.01+wet*0.12);
 }
}

/// Attach lights after Bevy has instantiated the glTF, so they follow the fold.
pub fn attach_lights(mut c: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>,
    parents: Query<&ChildOf>, roots: Query<(),With<ImportedHornet>>, nodes: Query<(Entity,&Name),Added<Name>>) {
    for (entity,name) in &nodes {
        if name.as_str()=="HornetGearNose" && in_import(entity,&parents,&roots) {
            let light=c.spawn((PointLight{color:Color::srgb(1.,0.95,0.82),intensity:0.,range:18.,shadows_enabled:false,..default()},Transform::from_xyz(1.,-1.5,0.),Lamp(Aircraft::Hornet,2))).id();
            c.entity(entity).add_child(light);
            continue;
        }
        let (color,z)=match name.as_str() {
            "HornetWingTipPivotL" => (Color::srgb(0.01,1.,0.15),19.),
            "HornetWingTipPivotR" => (Color::srgb(1.,0.02,0.01),-19.),
            _=>continue,
        };
        if !in_import(entity,&parents,&roots) {continue;}
        let lens_material=mats.add(StandardMaterial{base_color:color,emissive:color.to_linear()*15.,..default()});
        let lens=part(&mut c,&mut meshes,entity,Sphere::new(0.65).mesh().uv(12,8),lens_material,Transform::from_xyz(-4.,0.3,z));
        c.entity(lens).insert(Lens(Aircraft::Hornet));
        let light=c.spawn((PointLight{color,intensity:0.,range:7.,shadows_enabled:false,..default()},Transform::from_xyz(-4.,0.3,z),Lamp(Aircraft::Hornet,0))).id();
        c.entity(entity).add_child(light);
    }
}

fn in_import(mut entity: Entity, parents: &Query<&ChildOf>, roots: &Query<(), With<ImportedHornet>>) -> bool {
    loop {
        if roots.get(entity).is_ok() { return true; }
        let Ok(parent)=parents.get(entity) else { return false; };
        entity=parent.parent();
    }
}

pub fn update(f: Res<Flight>, view: Res<crate::ViewState>, time: Res<Time>, mut hook: Local<f32>, mut wheel_spin: Local<f32>, parents: Query<&ChildOf>,
    roots: Query<(),With<ImportedHornet>>, mut nodes: Query<(Entity,&Name,&mut Transform,&mut Visibility),Without<ImportedHornet>>) {
    if f.vehicle != Vehicle::Gripen || f.aircraft != Aircraft::Hornet { return; }
    let desired=if f.systems.hook_down {0.65} else {0.};
    let dt=if f.paused {0.} else {time.delta_secs().min(0.1)};
    if f.hornet.initialized {*hook=f.hornet.hook_angle as f32;}else{*hook += (desired-*hook).clamp(-0.32*dt,0.32*dt);}
    if f.landed && f.gear_extension > 0.98 {
        *wheel_spin = (*wheel_spin - f.velocity.length() as f32*dt/0.32).rem_euclid(std::f32::consts::TAU);
    }
    let angles=super::super::angles(&f);
    let retract=(1.-f.gear_extension.clamp(0.,1.)) as f32;
    for (entity,name,mut transform,mut visible) in &mut nodes {
        let name=name.as_str();
        if !(name.starts_with("Hornet") || name.starts_with("F18-landing") || name.starts_with("F18-cockpit") || name.starts_with("F18-hud") || name.starts_with("F18-airframe") || name.starts_with("F18-canopy")) || !in_import(entity,&parents,&roots) {continue;}
        let rotation=match name {
            "HornetCanopyPivot" | "HornetCanopyFrame" => Some(Quat::from_rotation_z(f.systems.hornet_canopy as f32*0.48)),
            "HornetHookPivot" => Some(Quat::from_rotation_z(*hook)),
            "HornetWingTipPivotL" => Some(Quat::from_rotation_x(-(f.systems.hornet_wing_fold as f32)*1.3)),
            "HornetWingTipPivotR" => Some(Quat::from_rotation_x((f.systems.hornet_wing_fold as f32)*1.3)),
            "HornetMainDoorL" => Some(Quat::from_rotation_x(-(f.gear_door as f32)*1.2)),
            "HornetMainDoorR" => Some(Quat::from_rotation_x((f.gear_door as f32)*1.2)),
            "HornetNoseDoorL" => Some(Quat::from_rotation_x(-(f.gear_door as f32)*1.2)),
            "HornetNoseDoorR" => Some(Quat::from_rotation_x((f.gear_door as f32)*1.2)),
            "HornetNoseBrace" => Some(Quat::from_rotation_z(retract*0.55)),
            "HornetGearNose" => Some(Quat::from_rotation_z(retract*1.35)),
            "HornetGearMainL" => Some(Quat::from_rotation_z(-retract*0.35)*Quat::from_rotation_x(-retract*1.35)),
            "HornetGearMainR" => Some(Quat::from_rotation_z(-retract*0.35)*Quat::from_rotation_x(retract*1.35)),
            "HornetWheelNoseL" | "HornetWheelNoseR" => Some(Quat::from_rotation_y((if f.hornet.initialized{f.hornet.nose_steering as f32}else{-f.controls.y as f32*0.35/(1.+f.airspeed as f32*0.1)})*f.gear_extension as f32)*Quat::from_rotation_z(*wheel_spin)),
            "HornetWheelMainL" | "HornetWheelMainR" => Some(Quat::from_rotation_z(*wheel_spin)),
            _ if name.starts_with("HornetPanel") => name.strip_prefix("HornetPanel").and_then(|s|s.parse::<usize>().ok()).filter(|i|*i<angles.len()).map(|i| {
                let value=crate::control_surfaces::effective_angle(&f.damage,i,angles[i]) as f32;
                if i==3 || i==7 {Quat::from_rotation_y(value)} else {Quat::from_rotation_z(if i==0 || i==4 {-value}else{value})}
            }),
            _=>None,
        };
        if let Some(rotation)=rotation { transform.rotation=rotation; }
        if let Some(pose)=gear_motion::pose(name,f.gear_extension as f32,f.gear_door as f32) {
            if name.starts_with("HornetStrut") { *transform=pose; }
            else if name.starts_with("HornetWheel") { transform.translation=pose.translation; transform.rotation=pose.rotation*transform.rotation; }
            else { transform.rotation=pose.rotation; }
        }
        let wheel_index=match name {
            "HornetWheelNoseL" | "HornetWheelNoseR" => Some(0),
            "HornetWheelMainL" => Some(2),
            "HornetWheelMainR" => Some(1),
            _=>None,
        };
        if let Some(i)=wheel_index {
            transform.translation+=Vec3::Y*(f.gear_compression[i] as f32*10.);
        }
        // Keep the retracted assembly visible while the doors are moving.
        // The swap happens only once the aircraft is fully retracted and the
        // doors are fully closed; opening the doors reveals it again.
        let stowed = f.gear_extension <= 1e-9 && f.gear_door <= 1e-9;
        let show=if name.starts_with("F18-landingOff") {Some(stowed)}
            else if name.starts_with("F18-cockpit") || name.starts_with("F18-hud") || name.starts_with("F18-airframe") || name.starts_with("F18-canopy") || name=="HornetFixedWindscreen" || name=="HornetCanopyFrame" {Some(!(view.index==3 && !view.map && !view.following()))}
            else if name.starts_with("F18-landingOnLight") {Some(f.gear_extension>0.99 && f.lights && f.systems.hornet_landing_light)}
            // Retracted gear stays inside the bay; never pop it out of existence.
            else if name.starts_with("HornetGear") && !name.ends_with("Mesh") {Some(true)}
            else if let Some(i)=name.strip_prefix("HornetPanel").and_then(|s|s.parse::<usize>().ok()).filter(|i|*i<f.damage.surface_state.len()) {
                Some(f.damage.surface_state[i]!=crate::combat::damage::SurfaceState::Detached)
            }
            else {None};
        if let Some(show)=show {visible.set_if_neq(if show {Visibility::Inherited} else {Visibility::Hidden});}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stabilator_nose_up_raises_trailing_edges() {
        let mut app=App::new();let mut f=Flight::runway();f.aircraft=Aircraft::Hornet;
        f.hornet.initialized=true;f.hornet.angles[0]=0.2;f.hornet.angles[4]=0.2;
        app.insert_resource(f).init_resource::<crate::ViewState>().init_resource::<Time>().add_systems(Update,update);
        let root=app.world_mut().spawn((ImportedHornet,Transform::default(),Visibility::Inherited)).id();
        let panels=[0,4].map(|i|app.world_mut().spawn((Name::new(format!("HornetPanel{i}")),Transform::default(),Visibility::Inherited,ChildOf(root))).id());
        app.update();
        for panel in panels {let q=app.world().get::<Transform>(panel).unwrap().rotation;assert!((q*Vec3::NEG_X).y>0.,"trailing edge must rise for nose up");}
    }
    #[test]
    fn heat_haze_tracks_simulation_time_and_engine_state() {
        let mut app=App::new();let mut f=Flight::runway();f.aircraft=Aircraft::Hornet;
        f.engine=true;f.thrust=10000.;f.spool=0.7;f.systems.engine_on=[true;2];f.systems.engines=[1.;2];
        app.insert_resource(f).init_resource::<Assets<Mesh>>().add_systems(Update,heat_haze);
        let mesh=app.world_mut().resource_mut::<Assets<Mesh>>().add(heat_haze_mesh(0.,0));
        let e=app.world_mut().spawn((HornetHeatHaze(0),Mesh3d(mesh.clone()),Transform::default(),Visibility::Inherited)).id();
        app.update();
        let read=|app:&App|app.world().resource::<Assets<Mesh>>().get(&mesh).unwrap().attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().as_float3().unwrap().to_vec();
        let first=read(&app);app.update();assert_eq!(first,read(&app));
        app.world_mut().resource_mut::<Flight>().elapsed=0.1;app.update();assert_ne!(first,read(&app));
        assert_eq!(*app.world().get::<Visibility>(e).unwrap(),Visibility::Inherited);
        app.world_mut().resource_mut::<Flight>().systems.engine_on[0]=false;app.update();
        assert_eq!(*app.world().get::<Visibility>(e).unwrap(),Visibility::Hidden);
    }

    #[test]
    fn exhaust_radiance_animates_without_cross_engine_failure_leakage() {
        let mut app=App::new();let mut f=Flight::runway();f.aircraft=Aircraft::Hornet;
        f.engine=true;f.thrust=10000.;f.spool=1.;f.systems.engine_on=[false,true];f.systems.engines=[0.,1.];
        app.insert_resource(f).init_resource::<Assets<StandardMaterial>>().add_systems(Update,exhaust);
        let m=app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial::default());
        let e=app.world_mut().spawn((HornetExhaust(1,0),Transform::default(),Visibility::Inherited,MeshMaterial3d(m.clone()))).id();
        app.update();
        assert_eq!(*app.world().get::<Visibility>(e).unwrap(),Visibility::Inherited);
        let first=app.world().resource::<Assets<StandardMaterial>>().get(&m).unwrap().base_color;
        app.update();
        assert_eq!(first,app.world().resource::<Assets<StandardMaterial>>().get(&m).unwrap().base_color);
        app.world_mut().resource_mut::<Flight>().elapsed=2.375;app.update();
        assert_ne!(first,app.world().resource::<Assets<StandardMaterial>>().get(&m).unwrap().base_color);
        app.world_mut().resource_mut::<Flight>().crashed=true;app.update();
        assert_eq!(*app.world().get::<Visibility>(e).unwrap(),Visibility::Hidden);
    }

    #[test]
    fn exhaust_distinguishes_dry_burner_and_failed_engine() {
        let mut app=App::new();let mut f=Flight::runway();f.aircraft=Aircraft::Hornet;
        f.engine=true;f.thrust=10000.;f.spool=0.7;f.systems.engine_on=[true;2];f.systems.engines=[1.;2];
        app.insert_resource(f).init_resource::<Assets<StandardMaterial>>().add_systems(Update,exhaust);
        let m=app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial::default());
        let core=app.world_mut().spawn((HornetExhaust(0,0),Transform::default(),Visibility::Inherited,MeshMaterial3d(m.clone()))).id();
        let cell_material=app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial::default());
        let cell=app.world_mut().spawn((HornetExhaust(0,7),Transform::default(),Visibility::Inherited,MeshMaterial3d(cell_material))).id();
        let inner_material=app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial::default());
        let inner=app.world_mut().spawn((HornetExhaust(0,13),Transform::default(),Visibility::Inherited,MeshMaterial3d(inner_material))).id();
        let light=app.world_mut().spawn((HornetEngineGlow(0),PointLight::default())).id();
        app.update();
        for spool in [0.25,0.55,0.85] {
            app.world_mut().resource_mut::<Flight>().spool=spool; app.update();
            assert_eq!(*app.world().get::<Visibility>(core).unwrap(),Visibility::Hidden);
            assert_eq!(*app.world().get::<Visibility>(inner).unwrap(),Visibility::Inherited);
            assert!(app.world().get::<Transform>(inner).unwrap().translation.z<0.);
            assert!(app.world().get::<PointLight>(light).unwrap().intensity>0.);
            let material=app.world().resource::<Assets<StandardMaterial>>().get(&m).unwrap();
            assert!(material.base_color.alpha()>0.17);
            assert!(material.emissive.red>0.01);
            assert_eq!(*app.world().get::<Visibility>(cell).unwrap(),Visibility::Hidden);
        }
        let dry=app.world().get::<Transform>(core).unwrap().scale.z;
        assert_eq!(*app.world().get::<Visibility>(cell).unwrap(),Visibility::Hidden);
        app.world_mut().resource_mut::<Flight>().spool=1.;app.update();
        assert!(app.world().get::<Transform>(core).unwrap().scale.z>dry*3.);
        assert_eq!(*app.world().get::<Visibility>(cell).unwrap(),Visibility::Inherited);
        app.world_mut().resource_mut::<Flight>().systems.engine_on[0]=false;app.update();
        assert_eq!(*app.world().get::<Visibility>(core).unwrap(),Visibility::Hidden);
        assert_eq!(*app.world().get::<Visibility>(inner).unwrap(),Visibility::Hidden);
        assert_eq!(app.world().get::<PointLight>(light).unwrap().intensity,0.);
        assert_eq!(*app.world().get::<Visibility>(cell).unwrap(),Visibility::Hidden);
    }
    #[test]
    fn only_imported_rig_uses_the_actual_gear_and_control_state() {
        let mut app=App::new();
        let mut f=Flight::runway();
        f.aircraft=Aircraft::Hornet;
        f.gear_extension=0.5;
        f.flaps=0.75;
        app.insert_resource(f).init_resource::<crate::ViewState>().init_resource::<Time>().add_systems(Update,update);
        let root=app.world_mut().spawn((ImportedHornet,Transform::default(),Visibility::Inherited)).id();
        let panel=app.world_mut().spawn((Name::new("HornetPanel1"),Transform::default(),Visibility::Inherited,ChildOf(root))).id();
        let nose=app.world_mut().spawn((Name::new("HornetGearNose"),Transform::default(),Visibility::Inherited,ChildOf(root))).id();
        let door=app.world_mut().spawn((Name::new("HornetMainDoorL"),Transform::default(),Visibility::Inherited,ChildOf(root))).id();
        let nose_door=app.world_mut().spawn((Name::new("HornetNoseDoorR"),Transform::default(),Visibility::Inherited,ChildOf(root))).id();
        let closed=app.world_mut().spawn((Name::new("F18-landingOff_4"),Transform::default(),Visibility::Inherited,ChildOf(root))).id();
        let wheel=app.world_mut().spawn((Name::new("HornetWheelNoseL"),Transform::from_xyz(1.,2.,3.),Visibility::Inherited,ChildOf(root))).id();
        let unrelated=app.world_mut().spawn((Name::new("HornetGearNose"),Transform::default(),Visibility::Inherited)).id();
        app.update();
        assert_ne!(app.world().get::<Transform>(panel).unwrap().rotation,Quat::IDENTITY);
        assert_eq!(app.world().get::<Transform>(nose).unwrap().rotation,Quat::IDENTITY);
        assert_eq!(app.world().get::<Transform>(door).unwrap().rotation,Quat::IDENTITY);
        app.world_mut().resource_mut::<Flight>().gear_door=1.;
        app.update();
        assert_ne!(app.world().get::<Transform>(door).unwrap().rotation,Quat::IDENTITY);
        assert_ne!(app.world().get::<Transform>(nose_door).unwrap().rotation,Quat::IDENTITY);
        app.world_mut().resource_mut::<Flight>().gear_door=0.;
        app.update();
        assert_eq!(app.world().get::<Transform>(door).unwrap().rotation,Quat::IDENTITY);
        assert_eq!(app.world().get::<Transform>(nose_door).unwrap().rotation,Quat::IDENTITY);
        assert_eq!(*app.world().get::<Visibility>(closed).unwrap(),Visibility::Hidden);
        assert_eq!(app.world().get::<Transform>(unrelated).unwrap().rotation,Quat::IDENTITY);
        let uncompressed=app.world().get::<Transform>(wheel).unwrap().translation;
        app.world_mut().resource_mut::<Flight>().gear_compression[0]=0.1;
        app.update();
        assert_eq!(app.world().get::<Transform>(wheel).unwrap().translation,uncompressed+Vec3::Y);
        app.world_mut().resource_mut::<Flight>().damage.surface_state[1]=crate::combat::damage::SurfaceState::Detached;
        app.update();
        assert_eq!(*app.world().get::<Visibility>(panel).unwrap(),Visibility::Hidden);
        app.world_mut().resource_mut::<Flight>().gear_extension=0.;
        app.update();
        assert_eq!(*app.world().get::<Visibility>(nose).unwrap(),Visibility::Inherited);
        assert_eq!(*app.world().get::<Visibility>(closed).unwrap(),Visibility::Inherited);
        // At either end of the gear stroke, opening or closing doors must
        // not swap to the closed placeholder before they finish moving.
        app.world_mut().resource_mut::<Flight>().gear_door=0.5;
        app.update();
        assert_eq!(*app.world().get::<Visibility>(nose).unwrap(),Visibility::Inherited);
        assert_eq!(*app.world().get::<Visibility>(closed).unwrap(),Visibility::Hidden);
        app.world_mut().resource_mut::<Flight>().gear_door=0.;
        app.update();
        assert_eq!(*app.world().get::<Visibility>(nose).unwrap(),Visibility::Inherited);
        assert_eq!(*app.world().get::<Visibility>(closed).unwrap(),Visibility::Inherited);
    }
    #[test]
    fn navigation_lights_are_children_of_the_moving_wing_pivots() {
        let mut app=App::new();
        app.init_resource::<Assets<Mesh>>().init_resource::<Assets<StandardMaterial>>().add_systems(Update,attach_lights);
        let root=app.world_mut().spawn(ImportedHornet).id();
        let wing=app.world_mut().spawn((Name::new("HornetWingTipPivotL"),ChildOf(root))).id();
        let unrelated=app.world_mut().spawn(Name::new("HornetWingTipPivotR")).id();
        app.update();
        let world=app.world_mut();
        let attached=world.query::<(&Lamp,&ChildOf)>().iter(world).filter(|(lamp,parent)|lamp.0==Aircraft::Hornet && parent.parent()==wing).count();
        assert_eq!(attached,1);
        assert!(world.get::<Children>(unrelated).is_none());
    }
}
