//! Hornet cannon presentation, driven by actual ammunition expenditure.
//! Fixed meshes/puff slots; this module never mutates flight or weapon physics.
use super::*;
use bevy::{camera::primitives::Aabb,mesh::VertexAttributeValues as V};
#[derive(Component)] pub struct CannonVisual(u8);
#[derive(Component)] pub struct CannonLamp;
#[derive(Clone,Copy)] struct Puff {position:DVec3,velocity:DVec3,born:f64}
pub struct FireState {generation:u64,ammo:Option<u16>,elapsed:f64,last_shot:Option<f64>,puffs:Vec<Puff>}
impl Default for FireState {fn default()->Self {Self{generation:u64::MAX,ammo:None,elapsed:0.,last_shot:None,puffs:Vec::with_capacity(24)}}}
impl FireState {
    fn observe(&mut self,ammo:u16,capacity:u16,clock:f64,next:f64,interval:f64,elapsed:f64,generation:u64)->bool {
        if self.generation!=generation || elapsed<self.elapsed {self.generation=generation;self.ammo=None;self.last_shot=None;self.puffs.clear();}
        let spent=self.ammo.is_some_and(|old|ammo<old);
        let recent=self.ammo.is_none()&&ammo<capacity&&next>clock&&next-clock<=interval+1e-8;
        self.ammo=Some(ammo);self.elapsed=elapsed;
        if spent||recent {self.last_shot=Some(elapsed-(clock-(next-interval)).max(0.));}
        spent||recent
    }
    fn pulse(&self,now:f64)->f32 {
        self.last_shot.map_or(0.,|t|{let age=(now-t).max(0.);if age<0.055 {(1.-age/0.055).powf(0.45) as f32}else{0.}})
    }
}
fn dynamic_mesh(capacity:usize)->Mesh {
    let mut m=super::mesh(vec![],vec![]);
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION,Vec::<[f32;3]>::with_capacity(capacity));
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR,Vec::<[f32;4]>::with_capacity(capacity));
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL,Vec::<[f32;3]>::with_capacity(capacity));
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0,Vec::<[f32;2]>::with_capacity(capacity));
    m.insert_indices(Indices::U32(Vec::with_capacity(capacity/4*6)));m
}
fn flash_mesh()->Mesh {
    let mut p=Vec::new();let mut colors=Vec::new();let mut ix=Vec::new();
    for (ring,(z,r)) in [(0.,0.055),(0.18,0.20),(0.55,0.12),(1.25,0.)].into_iter().enumerate() {
        for j in 0..18 {
            let a=j as f32*std::f32::consts::TAU/18.;let radial=r*(1.+0.4*(a*6.).cos());
            p.push(Vec3::new(a.cos()*radial,a.sin()*radial,-z));
            colors.push(match ring {0=>[40.,32.,16.,0.95],1=>[24.,8.,1.5,0.85],2=>[9.,1.5,0.1,0.45],_=>[2.,0.16,0.01,0.]});
            if ring>0 {let b=(ring*18+j)as u32;let n=(ring*18+(j+1)%18)as u32;ix.extend([b-18,n-18,n,b-18,n,b]);}
        }
    }
    let mut mesh=super::mesh(p,ix);mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR,colors);mesh
}
pub fn setup(mut commands:Commands,mut meshes:ResMut<Assets<Mesh>>,mut mats:ResMut<Assets<StandardMaterial>>,shared:Res<aim9x::VisualAssets>) {
    let glow=mats.add(StandardMaterial{base_color:Color::WHITE,unlit:true,alpha_mode:AlphaMode::Add,cull_mode:None,..default()});
    let mut smoke=mats.get(&shared.naval_smoke).unwrap().clone();smoke.base_color=Color::srgba(0.62,0.65,0.68,0.42);
    let smoke=mats.add(smoke);
    let mut tracer=mats.get(&glow).unwrap().clone();
    tracer.base_color_texture=mats.get(&shared.smoke).unwrap().base_color_texture.clone();
    let tracer=mats.add(tracer);
    for (kind,mesh,material) in [(0,flash_mesh(),glow),(1,dynamic_mesh(128*12),tracer),(2,dynamic_mesh(24*4),smoke)] {
        commands.spawn((CannonVisual(kind),Mesh3d(meshes.add(mesh)),MeshMaterial3d(material),Transform::default(),Visibility::Hidden,
            bevy::light::NotShadowCaster,bevy::light::NotShadowReceiver,bevy::camera::visibility::NoAutoAabb,
            Aabb::from_min_max(Vec3::splat(-1.5),Vec3::splat(1.5))));
    }
    commands.spawn((CannonLamp,PointLight{color:Color::srgb(1.,0.48,0.12),intensity:0.,range:6.,radius:0.05,shadows_enabled:false,..default()},Transform::default()));
}
fn quad(p:&mut Vec<[f32;3]>,colors:&mut Vec<[f32;4]>,uv:&mut Vec<[f32;2]>,center:Vec3,along:Vec3,across:Vec3,half:f32,width:f32,color:[f32;4]) {
    for (x,y) in [(-1.,-1.),(1.,-1.),(1.,1.),(-1.,1.)] {p.push((center+along*half*y+across*width*x).to_array());colors.push(color);uv.push([(x+1.)*0.5,(y+1.)*0.5]);}
}
pub fn update(f:Res<Flight>,c:Res<Combat>,view:Res<ViewState>,menu:Res<crate::start_screen::Menu>,
    cameras:Query<&Transform,(With<FlightCamera>,Without<CannonVisual>,Without<CannonLamp>)>,
    mut state:Local<FireState>,mut meshes:ResMut<Assets<Mesh>>,
    mut visuals:Query<(&CannonVisual,&Mesh3d,&mut Transform,&mut Visibility,&mut Aabb),Without<FlightCamera>>,
    mut lamp:Query<(&mut PointLight,&mut Transform),(With<CannonLamp>,Without<CannonVisual>,Without<FlightCamera>)>) {
    if menu.open {
        for (_,_,_,mut visibility,_) in &mut visuals {*visibility=Visibility::Hidden;}
        for (mut light,_) in &mut lamp {light.intensity=0.;}return;
    }
    let eligible=f.aircraft==crate::aircraft::Aircraft::Hornet&&f.vehicle==Vehicle::Gripen&&c.active&&c.fight.is_some();
    let mut pulse=0.;
    let muzzle=f.position+f.orientation*crate::aircraft::gun_origin(f.aircraft);
    if eligible {if let Some(fight)=&c.fight {
        let first=state.ammo.is_none()||state.generation!=view.generation;
        let shot=state.observe(fight.gun,f.aircraft.spec().gun,c.clock,fight.gun_at,f.aircraft.spec().gun_interval,f.elapsed,view.generation);
        if first {
            for round in fight.rounds.iter().filter(|r|!r.hostile&&r.age<0.45).take(24) {
                state.puffs.push(Puff{position:round.position-round.velocity*round.age-f.up()*(4.905*round.age*round.age),velocity:f.velocity,born:f.elapsed-round.age});
            }
        }
        if shot&&!f.crashed&&!f.ejection.active {
            if let Some(born)=state.last_shot.filter(|t|f.elapsed-*t<0.45) {
                if state.puffs.len()==24 {state.puffs.remove(0);}
                state.puffs.push(Puff{position:muzzle-f.velocity*(f.elapsed-born),velocity:f.velocity,born});
            }
        }
        state.puffs.retain(|p|f.elapsed-p.born<0.45&&f.elapsed>=p.born);
        if !f.crashed&&!f.ejection.active {pulse=state.pulse(f.elapsed);}
    }}else {state.ammo=None;state.puffs.clear();state.last_shot=None;}
    let camera=cameras.iter().next().copied().unwrap_or_default();
    let right=camera.rotation*Vec3::X;let up=camera.rotation*Vec3::Y;
    for (mut light,mut t) in &mut lamp {
        light.intensity=if view.map {0.}else{18000.*pulse};t.translation=world::local(muzzle+f.forward()*0.2,&f);
    }
    for (tag,handle,mut t,mut visibility,mut bounds) in &mut visuals {
        if !eligible||view.map {*visibility=Visibility::Hidden;continue;}
        if tag.0==0 {
            *visibility=if pulse>0. {Visibility::Inherited}else{Visibility::Hidden};
            let flicker=0.85+0.15*(c.clock*317.).sin().abs() as f32;
            *t=Transform{translation:world::local(muzzle+f.forward()*0.02,&f),rotation:world::local_rot(f.orientation*DQuat::from_rotation_z(c.clock*29.),&f),
                scale:Vec3::new(0.65+0.35*pulse,0.65+0.35*pulse,(0.65+0.35*pulse)*flicker)};
            continue;
        }
        let has_work=if tag.0==1 {c.fight.as_ref().is_some_and(|fight|fight.rounds.iter().any(|r|!r.hostile&&r.alive))}else{!state.puffs.is_empty()};
        if !has_work {*visibility=Visibility::Hidden;continue;}
        let Some(mesh)=meshes.get_mut(&handle.0)else{continue;};
        let mut p=match mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION){Some(V::Float32x3(v))=>v,_=>Vec::new()};
        let mut colors=match mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR){Some(V::Float32x4(v))=>v,_=>Vec::new()};
        let mut uv=match mesh.remove_attribute(Mesh::ATTRIBUTE_UV_0){Some(V::Float32x2(v))=>v,_=>Vec::new()};p.clear();colors.clear();uv.clear();
        if tag.0==1 {
            for round in c.fight.iter().flat_map(|fight|fight.rounds.iter()).filter(|r|!r.hostile&&r.alive).take(128) {
                let tip=world::local(round.position,&f);let distance=tip.distance(camera.translation);if distance>6000.{continue;}
                let direction=(world::frame(&f).conjugate()*round.velocity.normalize_or_zero()).as_vec3();
                let across=direction.cross((camera.translation-tip).normalize_or(Vec3::Y)).normalize_or(right);
                let width=(distance*0.0006).clamp(0.012,0.6);let fade=(1.-round.age as f32/2.5).clamp(0.,1.);
                for (length,w,color) in [(18.,width*1.9,[6.,1.3,0.12,0.18*fade]),(12.,width,[24.,12.,1.5,0.65*fade]),(4.,width*0.35,[40.,35.,18.,0.95*fade])] {
                    let length=(length as f32).min((round.velocity.length()*round.age) as f32);
                    quad(&mut p,&mut colors,&mut uv,tip-direction*length*0.5,direction,across,length*0.5,w,color);
                }
            }
        } else {
            for puff in &state.puffs {
                let age=(f.elapsed-puff.born).max(0.);let inherited=0.08*(1.-(-age/0.08).exp());
                let center=puff.position+puff.velocity*inherited+f.wind*(age-inherited)+f.up()*age*0.3;
                let center=world::local(center,&f);if center.distance_squared(camera.translation)>800_f32.powi(2){continue;}
                let radius=(0.055+age*0.85) as f32;let alpha=((1.-age/0.45).powi(2)*(age/0.01).clamp(0.,1.)) as f32;
                quad(&mut p,&mut colors,&mut uv,center,up,right,radius*1.25,radius,[0.9,0.93,0.96,alpha]);
            }
        }
        *visibility=if p.is_empty(){Visibility::Hidden}else{Visibility::Inherited};
        *bounds=Aabb::enclosing(p.iter().map(|v|Vec3::from(*v))).unwrap_or_else(||Aabb::from_min_max(Vec3::ZERO,Vec3::ONE));
        let count=p.len();let mut normals=match mesh.remove_attribute(Mesh::ATTRIBUTE_NORMAL){Some(V::Float32x3(v))=>v,_=>Vec::new()};normals.resize(count,[0.,0.,1.]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION,p);mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,normals);mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR,colors);mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,uv);
        if let Some(Indices::U32(ix))=mesh.indices_mut(){if ix.len()!=count/4*6 {ix.clear();for i in 0..count/4 {let k=i as u32*4;ix.extend([k,k+1,k+2,k,k+2,k+3]);}}}
    }
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn muzzle_mesh_is_finite_and_inside_its_fixed_bounds(){
        let m=flash_mesh();
        let Some(V::Float32x3(p))=m.attribute(Mesh::ATTRIBUTE_POSITION)else{panic!()};
        assert_eq!(p.len(),72);assert!(p.iter().flatten().all(|v|v.is_finite()&&v.abs()<=1.5));
        assert!(m.indices().unwrap().iter().all(|i|i<p.len()));
        let Some(V::Float32x4(c))=m.attribute(Mesh::ATTRIBUTE_COLOR)else{panic!()};
        assert!(c.iter().all(|v|v.iter().all(|x|x.is_finite())&&v[3]>=0.&&v[3]<=1.));
    }
    #[test]fn flash_follows_real_expenditure_and_expires_without_empty_or_reload_ghosts(){
        let mut state=FireState::default();assert!(!state.observe(412,412,0.,0.,0.02,0.,1));assert_eq!(state.pulse(0.),0.);
        assert!(state.observe(411,412,0.1,0.12,0.02,0.1,1));assert!(state.pulse(0.1)>0.9);
        assert!(!state.observe(411,412,0.11,0.12,0.02,0.11,1));assert_eq!(state.pulse(0.2),0.);
        assert!(!state.observe(412,412,0.3,0.,0.02,0.3,1));
        assert!(!state.observe(0,412,10.,0.,0.02,10.,2));assert_eq!(state.pulse(10.),0.);
        assert!(state.observe(0,412,10.1,10.12,0.02,10.1,3));assert!(state.pulse(10.1)>0.9);
    }
}
