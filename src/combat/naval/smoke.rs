//! Bounded presentation-only launcher clouds; no per-launch entities or lights.
use super::*;
use bevy::{camera::primitives::Aabb,mesh::VertexAttributeValues as V};
#[derive(Component)] pub struct LaunchCloudMesh;
#[derive(Clone,Copy)] struct Cloud {position:DVec3,velocity:DVec3,born:f64,seed:u64}
pub struct Clouds {generation:u64,seen:[u64;WEAPON_LIMIT],clouds:Vec<Cloud>}
impl Default for Clouds {fn default()->Self{Self{generation:u64::MAX,seen:[0;WEAPON_LIMIT],clouds:Vec::with_capacity(32)}}}
impl Clouds {
    fn sync(&mut self,n:&State,f:&Flight,generation:u64) {
        if self.generation!=generation {self.generation=generation;self.seen.fill(0);self.clouds.clear();}
        self.clouds.retain(|p|n.clock-p.born<12. && n.clock>=p.born);
        for (i,m) in n.rounds.iter().enumerate() {
            if self.seen[i]==m.id {continue;}self.seen[i]=m.id;
            let TargetId::Ship(ship)=m.source else{continue;};
            if m.age>=12. || !matches!(m.kind,Kind::Standard|Kind::Essm|Kind::AntiShip) {continue;}
            let born=n.clock-m.age;
            let position=carrier::ship_world(f,ship,super::weapons::launch_mount(ship),f.elapsed-m.age);
            let velocity=carrier::ship_velocity(f,ship,position,f.elapsed-m.age);
            if self.clouds.len()==32 {self.clouds.remove(0);}
            self.clouds.push(Cloud{position,velocity,born,seed:m.id});
        }
    }
}
pub fn setup(mut c:Commands,mut meshes:ResMut<Assets<Mesh>>,assets:Res<aim9x::VisualAssets>) {
    let mut mesh=super::super::mesh(vec![],vec![]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION,Vec::<[f32;3]>::with_capacity(1536));
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,Vec::<[f32;3]>::with_capacity(1536));
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR,Vec::<[f32;4]>::with_capacity(1536));
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,Vec::<[f32;2]>::with_capacity(1536));
    mesh.insert_indices(Indices::U32(Vec::with_capacity(2304)));
    c.spawn((LaunchCloudMesh,Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(assets.naval_smoke.clone()),Transform::default(),Visibility::Hidden,
        bevy::light::NotShadowCaster,bevy::light::NotShadowReceiver,
        bevy::camera::visibility::NoAutoAabb,Aabb::from_min_max(Vec3::ZERO,Vec3::ONE)));
}
pub fn update(f:Res<Flight>,c:Res<Combat>,view:Res<ViewState>,
    cameras:Query<&Transform,With<FlightCamera>>,mut state:Local<Clouds>,mut reported:Local<bool>,
    mut meshes:ResMut<Assets<Mesh>>,
    mut clouds:Query<(&Mesh3d,&mut Visibility,&mut Aabb),With<LaunchCloudMesh>>) {
    let Some(n)=c.naval.as_ref().filter(|_|c.active) else {
        state.clouds.clear();state.seen.fill(0);
        for (_,mut vis,_) in &mut clouds {vis.set_if_neq(Visibility::Hidden);}return;
    };
    state.sync(n,&f,view.generation);
    if state.clouds.is_empty() {
        for (_,mut vis,_) in &mut clouds {vis.set_if_neq(Visibility::Hidden);}return;
    }
    let camera=cameras.iter().next().copied().unwrap_or_default();
    let basis=world::frame(&f).conjugate();let origin=world::render_origin(&f);
    let right=camera.rotation*Vec3::X;let vertical=camera.rotation*Vec3::Y;
    for (handle,mut visible,mut bounds) in &mut clouds {
        let Some(mesh)=meshes.get_mut(&handle.0) else{continue;};
        let mut p=match mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION){Some(V::Float32x3(v))=>v,_=>Vec::new()};
        let mut color=match mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR){Some(V::Float32x4(v))=>v,_=>Vec::new()};
        let mut uv=match mesh.remove_attribute(Mesh::ATTRIBUTE_UV_0){Some(V::Float32x2(v))=>v,_=>Vec::new()};
        p.clear();color.clear();uv.clear();
        for cloud in &state.clouds {
            let age=(n.clock-cloud.born).max(0.);let up=cloud.position.normalize();
            // Exhaust initially inherits deck motion, then relaxes toward the
            // surrounding air. A fixed 20% inheritance left the launch cloud
            // behind the moving ship's bridge before it could spread visibly.
            let entrained=3.*(1.-(-age/3.).exp());
            let center=cloud.position+cloud.velocity*entrained+f.wind*(age-entrained)*0.65+up*(1.+age*0.65);
            let at=(basis*(center-origin)).as_vec3();
            let distance=at.distance_squared(camera.translation);if distance>12000_f32.powi(2){continue;}
            let lobes=if distance>2500_f32.powi(2){6}else{12};
            let growth=(age/2.).clamp(0.,1.).sqrt();let radius=(2.5+age*1.25) as f32;
            let fade=((1.-age/12.).clamp(0.,1.).powi(2)*(age/0.12).clamp(0.,1.)) as f32;
            let hot=(-age*2.4).exp() as f32;
            let tint=[0.93+hot*0.07,0.93-hot*0.16,0.93-hot*0.34,fade*0.72];
            for i in 0..lobes {
                let phase=i as f32*2.39996+cloud.seed as f32*0.73;
                let spread=(i as f32/lobes as f32).sqrt()*growth as f32;
                let middle=at+right*(phase.cos()*radius*1.5*spread)
                    +vertical*((0.35+0.65*phase.sin())*radius*spread);
                let size=radius*(0.7+0.25*(phase*1.7).sin());
                for (x,y) in [(-1.,-1.),(1.,-1.),(1.,1.),(-1.,1.)] {
                    p.push((middle+right*size*x+vertical*size*y).to_array());
                    color.push(tint);
                    uv.push([(x+1.)*0.5,(y+1.)*0.5]);
                }
            }
        }
        visible.set_if_neq(if p.is_empty(){Visibility::Hidden}else{Visibility::Inherited});
        *bounds=Aabb::enclosing(p.iter().map(|v|Vec3::from(*v))).unwrap_or_else(||Aabb::from_min_max(Vec3::ZERO,Vec3::ONE));
        let count=p.len();
        if !*reported && count>0 && std::env::var("NORD_NAVAL_CAPTURE").is_ok_and(|m|m.starts_with("defense-smoke-")) {
            println!("LAUNCH CLOUDS count={} vertices={} clock={} camera={:?} center={:?}",state.clouds.len(),count,n.clock,camera.translation,bounds.center);
            *reported=true;
        }
        let mut normals=match mesh.remove_attribute(Mesh::ATTRIBUTE_NORMAL){Some(V::Float32x3(v))=>v,_=>Vec::new()};
        normals.resize(count,[0.,0.,1.]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION,p);mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR,color);mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,uv);
        if let Some(Indices::U32(ix))=mesh.indices_mut() {
            if ix.len()!=count/4*6 {ix.clear();for i in 0..count/4 {let k=i as u32*4;ix.extend([k,k+1,k+2,k,k+2,k+3]);}}
        }
    }
}

pub(crate) fn review_mount()->DVec3 {super::weapons::launch_mount(1)}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn launcher_clouds_are_one_per_identity_bounded_and_resettable() {
        let f=flight(true);let mut c=Combat::default();let mut v=ViewState::default();
        let mut n=State::new(1,2,false);
        for id in 1..=40 {
            // Reuse one physical missile slot, as the real weapon pool does.
            let p=carrier::ship_world(&f,1,crate::combat::naval::weapons::launch_mount(1),0.);
            let mut m=Round{sam:Default::default(),id,source:TargetId::Ship(1),target:TargetId::Ship(0),kind:Kind::Standard,
                position:p,previous:p,velocity:p.normalize()*35.,age:0.4,alive:true,support_lost:0.,next_sensor:0.,visual:render_round(p,DVec3::ZERO)};
            m.visual.alive=true;n.rounds.clear();n.rounds.push(m);
            let mut single=Clouds::default();single.sync(&n,&f,1);single.sync(&n,&f,1);
            assert_eq!(single.clouds.len(),1);
        }
        c.naval=Some(n);let n=c.naval.as_mut().unwrap();n.clock=1.;
        let mut state=Clouds::default();
        for id in 1..=40 {n.rounds[0].id=id;state.sync(n,&f,0);}
        assert_eq!(state.clouds.len(),32);
        n.clock=20.;state.sync(n,&f,0);assert!(state.clouds.is_empty());
        v.generation=1;state.sync(n,&f,v.generation);assert_eq!(state.clouds.len(),1);
    }
}
