//! Fitted main weapons bay with four folding leaves and six eject launchers.
//! Hidden framing/linkage proportions are visual approximations.
use super::*;
use crate::aircraft::main_bay as bay;
use super::raptor_bay::{rigid,rod,block};
use super::raptor_detail::skin_y;

#[derive(Component)]
pub struct Leaf {pub side:f32,pub inner:bool}
#[derive(Component)]
pub struct Shoe(usize);
#[derive(Component)]
pub struct Arm(usize,f32,usize);
#[derive(Component)]
pub struct Elbow(usize,f32);
#[derive(Component)]
struct MatingHinge;

pub(super) fn cut(mut pieces:Vec<Vec<Vec3>>)->Vec<Vec<Vec3>> {
    // Piecewise-linear serrations, shared with the door and perimeter frame.
    for i in 0..16 {
        let x0=-bay::HALF+2.*bay::HALF*i as f32/16.;
        let x1=-bay::HALF+2.*bay::HALF*(i+1)as f32/16.;
        let f0=bay::edge(x0,false)-0.014;let f1=bay::edge(x1,false)-0.014;
        let a0=bay::edge(x0,true)+0.014;let a1=bay::edge(x1,true)+0.014;
        let fs=(f1-f0)/(x1-x0);let as_=(a1-a0)/(x1-x0);
        let planes=[(Vec3::X,x0-if i==0{0.006}else{0.}),(-Vec3::X,-x1-if i==15{0.006}else{0.}),
            (-Vec3::Y,-bay::ROOF),(Vec3::new(-fs,0.,1.),f0-fs*x0),(Vec3::new(as_,0.,-1.),-a0+as_*x0)];
        pieces=pieces.into_iter().flat_map(|mut p| {
            let mut outside=Vec::new();
            for (n,d) in planes {
                if p.len()<3{break;}
                let mut keep=Vec::new();let mut reject=Vec::new();
                for k in 0..p.len() {
                    let a=p[k];let b=p[(k+1)%p.len()];let da=n.dot(a)-d;let db=n.dot(b)-d;
                    if da>=0.{keep.push(a)}else{reject.push(a)}
                    if (da>=0.)!=(db>=0.){let q=a.lerp(b,da/(da-db));keep.push(q);reject.push(q);}
                }
                if reject.len()>=3{outside.push(reject);}p=keep;
            }
            outside
        }).collect();
    }
    pieces
}
fn xs(inner:bool)->Vec<f32> {
    let lo=if inner{0.006}else{bay::HALF*0.5+0.004};
    let hi=if inner{bay::HALF*0.5-0.004}else{bay::HALF-0.006};
    let mut v=vec![lo];v.extend((1..8).map(|i|bay::HALF*i as f32/8.).filter(|x|*x>lo&&*x<hi));v.push(hi);v
}
fn surface(side:f32,inner:bool,inside:bool)->Mesh {
    let mut v=Vec::new();let mut ix=Vec::new();let h=bay::pivot(side,inner);
    let xs=xs(inner);
    for ab in xs.windows(2) {for j in 0..28 {
        let at=|x:f32,t:f32| {
            let z=(bay::edge(x,false)+0.006)*(1.-t)+(bay::edge(x,true)-0.006)*t;
            let x=side*x;Vec3::new(x,skin_y(x,z,false)+if inside{0.016}else{-0.006},z)-h
        };
        let mut p=[at(ab[0],j as f32/28.),at(ab[1],j as f32/28.),at(ab[1],(j+1)as f32/28.),at(ab[0],(j+1)as f32/28.)];
        if (side>0.)==inside{p.swap(1,3);}quad(&mut v,&mut ix,p);
    }}
    let m=raptor_mesh::smooth(mesh(v,ix));
    if inside{m}else{raptor_mesh::paint_at(m,h)}
}
fn edge_shell(side:f32,inner:bool)->Mesh {
    let x=xs(inner);let mut points=Vec::new();
    for &u in &x{points.push((u,bay::edge(u,false)+0.006));}
    for &u in x.iter().rev(){points.push((u,bay::edge(u,true)-0.006));}
    let h=bay::pivot(side,inner);let mut v=Vec::new();let mut ix=Vec::new();
    for i in 0..points.len(){
        let (ax,az)=points[i];let (bx,bz)=points[(i+1)%points.len()];
        for j in 0..28 {
            let p=|t| {let x=side*(ax+(bx-ax)*t);let z=az+(bz-az)*t;Vec3::new(x,skin_y(x,z,false)-0.006,z)-h};
            let a=p(j as f32/28.);let b=p((j+1)as f32/28.);
            let mut q=[a,b,b+Vec3::Y*0.022,a+Vec3::Y*0.022];if side<0.{q.swap(1,3);}quad(&mut v,&mut ix,q);
        }
    }
    mesh(v,ix)
}
fn channel(c:&mut Commands,m:&mut Assets<Mesh>,root:Entity,mat:Handle<StandardMaterial>,a:Vec3,b:Vec3,w:f32) {
    channel_facing(c,m,root,mat,a,b,w,1.);
}
fn channel_facing(c:&mut Commands,m:&mut Assets<Mesh>,root:Entity,mat:Handle<StandardMaterial>,a:Vec3,b:Vec3,w:f32,facing:f32) {
    let rot=Quat::from_rotation_arc(Vec3::Z,(b-a).normalize());let length=a.distance(b);
    for (y,width,height) in [(0.,w,0.006),(0.016,0.006,0.032),(0.032,w*0.8,0.006)] {
        rigid(c,m,root,Cuboid::new(width,height,length).into(),mat.clone(),Transform::from_translation((a+b)*0.5+Vec3::Y*(y*facing)).with_rotation(rot));
    }
}
fn pin(c:&mut Commands,m:&mut Assets<Mesh>,root:Entity,mat:Handle<StandardMaterial>,at:Vec3,r:f32,len:f32) {
    rod(c,m,root,mat,at-Vec3::Z*len*0.5,at+Vec3::Z*len*0.5,r);
}
fn gearbox(c:&mut Commands,m:&mut Assets<Mesh>,root:Entity,coat:Handle<StandardMaterial>,metal:Handle<StandardMaterial>,at:Vec3) {
    let p=[(-0.07,-0.045),(-0.045,-0.065),(0.045,-0.065),(0.075,-0.035),(0.075,0.055),(0.04,0.085),(-0.04,0.085),(-0.07,0.055)]
        .map(|(x,y)|Vec3::new(x,y,-0.065));
    rigid(c,m,root,solid(&p,Vec3::Z*0.13),coat.clone(),Transform::from_translation(at));
    pin(c,m,root,metal.clone(),at,0.041,0.145);
    for k in 0..10 {let a=k as f32*std::f32::consts::TAU/10.;pin(c,m,root,metal.clone(),at+Vec3::new(a.cos()*0.054,a.sin()*0.054,0.076),0.0035,0.008);}
    for x in [-0.035,0.035] {rod(c,m,root,metal.clone(),at+Vec3::new(x,0.06,0.),at+Vec3::new(x,0.103,0.),0.009);}
}
pub(super) fn setup(c:&mut Commands,m:&mut Assets<Mesh>,mats:&mut Assets<StandardMaterial>,detail:&raptor_finish::Detail,root:Entity,skin:Handle<StandardMaterial>) {
    let coat=mats.add(StandardMaterial{base_color:Color::srgb(0.69,0.70,0.65),perceptual_roughness:0.67,..default()});let coat=detail.apply(mats,&coat);
    let frame=mats.add(StandardMaterial{base_color:Color::srgb(0.51,0.55,0.51),perceptual_roughness:0.72,..default()});let frame=detail.apply(mats,&frame);
    let metal=material(mats,Color::srgb(0.42,0.44,0.42),0.72);
    let seal=material(mats,Color::srgb(0.055,0.065,0.06),0.04);
    let skin=detail.apply(mats,&skin);
    // Closed roof and formed roof panels leave no view through the airframe.
    block(c,m,root,coat.clone(),Vec3::new(0.,bay::ROOF+0.01,(bay::FORE+bay::AFT)*0.5),Vec3::new(bay::HALF*2.,0.02,bay::AFT-bay::FORE));
    for x in [-0.86,-0.56,-0.26,0.26,0.56,0.86] {
        channel_facing(c,m,root,frame.clone(),Vec3::new(x,bay::ROOF-0.004,bay::FORE+0.12),Vec3::new(x,bay::ROOF-0.004,bay::AFT-0.12),0.034,-1.);
        for j in 0..14 {let z=bay::FORE+0.20+j as f32*0.30;rod(c,m,root,metal.clone(),Vec3::new(x,bay::ROOF-0.042,z),Vec3::new(x,bay::ROOF-0.038,z),0.0045);}
    }
    let mut wall_v=Vec::new();let mut wall_ix=Vec::new();
    for i in 0..16 {
        let a=-bay::HALF+2.*bay::HALF*i as f32/16.;let b=-bay::HALF+2.*bay::HALF*(i+1)as f32/16.;
        for aft in [false,true] {
            let z=|x|bay::edge(x,aft)+if aft{0.014}else{-0.014};
            let mut q=[Vec3::new(a,skin_y(a,z(a),false),z(a)),Vec3::new(b,skin_y(b,z(b),false),z(b)),Vec3::new(b,bay::ROOF,z(b)),Vec3::new(a,bay::ROOF,z(a))];
            if aft{q.swap(1,3);}quad(&mut wall_v,&mut wall_ix,q);
            let mid=(a+b)*0.5;pin(c,m,root,metal.clone(),Vec3::new(mid,skin_y(mid,z(mid),false)+0.04,z(mid)),0.003,0.009);
        }
    }
    for side in [-1.,1.] {
        let x=side*(bay::HALF+0.006);
        for j in 0..32 {let z0=bay::FORE+(bay::AFT-bay::FORE)*j as f32/32.;let z1=bay::FORE+(bay::AFT-bay::FORE)*(j+1)as f32/32.;
            let mut q=[Vec3::new(x,skin_y(x,z0,false),z0),Vec3::new(x,skin_y(x,z1,false),z1),Vec3::new(x,bay::ROOF,z1),Vec3::new(x,bay::ROOF,z0)];
            if side<0.{q.swap(1,3);}quad(&mut wall_v,&mut wall_ix,q);
        }
        if side>0.{for (z,r) in [(-1.5,0.008),(-0.8,0.006),(0.,0.008),(0.8,0.006),(1.6,0.008)] {
            channel_facing(c,m,root,frame.clone(),Vec3::new(-bay::HALF+0.025,bay::ROOF-0.004,z),Vec3::new(bay::HALF-0.025,bay::ROOF-0.004,z),r*3.,-1.);
        }}
        for k in 0..3 {
            let x=side*(bay::HALF-0.025-k as f32*0.018);
            rod(c,m,root,metal.clone(),Vec3::new(x,bay::ROOF-0.035,bay::FORE+0.15),Vec3::new(x,bay::ROOF-0.035,bay::AFT-0.15),0.004);
            for z in [-1.4,-0.5,0.5,1.6] {block(c,m,root,coat.clone(),Vec3::new(x,bay::ROOF-0.035,z),Vec3::new(0.014,0.018,0.028));}
        }
        for inner in [false,true] {
            let h=bay::pivot(side,inner);
            let leaf=c.spawn((Transform::from_translation(h),Visibility::Inherited,ChildOf(root),Leaf{side,inner})).id();
            rigid(c,m,leaf,surface(side,inner,false),skin.clone(),Transform::default());
            rigid(c,m,leaf,surface(side,inner,true),coat.clone(),Transform::default());
            rigid(c,m,leaf,edge_shell(side,inner),metal.clone(),Transform::default());
            let range=xs(inner);let lo=range[0]+0.025;let hi=range[range.len()-1]-0.025;
            for x in [lo,(lo+hi)*0.5,hi] {for j in 0..16 {
                let a=bay::FORE+0.14+(bay::AFT-bay::FORE-0.28)*j as f32/16.;let b=bay::FORE+0.14+(bay::AFT-bay::FORE-0.28)*(j+1)as f32/16.;
                let p=|z|Vec3::new(side*x,skin_y(side*x,z,false)+0.023,z)-h;
                channel(c,m,leaf,coat.clone(),p(a),p(b),0.023);
            }}
            for z in [-1.55,-1.05,-0.55,-0.05,0.45,0.95,1.45,2.0,2.35] {
                let p=|x:f32|Vec3::new(side*x,skin_y(side*x,z,false)+0.025,z)-h;
                channel(c,m,leaf,coat.clone(),p(lo),p(hi),0.025);
                for x in [lo,hi] {let at=p(x)+Vec3::Y*0.035;rod(c,m,leaf,metal.clone(),at,at+Vec3::Y*0.005,0.0035);}
            }
            // Longitudinal hinge journals are mounted on formed door brackets.
            for z in [-0.6,0.,0.6] {
                let axis=Vec3::Z*z;
                pin(c,m,leaf,metal.clone(),axis,0.016,0.085);
                let x=if inner{bay::HALF*0.5-0.025}else{bay::HALF-0.025};
                let attach=Vec3::new(side*x,skin_y(side*x,z,false)+0.045,z)-h;
                let e=rod(c,m,leaf,coat.clone(),attach,axis,0.011);c.entity(e).insert(MatingHinge);
            }
            if !inner {
                let fold=bay::pivot(side,true)-h;
                for z in [-0.6,0.,0.6]{let e=rod(c,m,leaf,metal.clone(),fold+Vec3::Z*(z-0.06),fold+Vec3::Z*(z+0.06),0.021);c.entity(e).insert(MatingHinge);}
                gearbox(c,m,leaf,coat.clone(),metal.clone(),fold+Vec3::new(0.,0.06,0.30));
            }
        }
        let h=bay::pivot(side,false);
        // Compact rotary drive, offset gearbox and journals, rather than a
        // large exposed linear cylinder passing through the moving leaves.
        gearbox(c,m,root,coat.clone(),metal.clone(),Vec3::new(side*(bay::HALF-0.055),-0.86,0.));
        let e=rod(c,m,root,metal.clone(),Vec3::new(side*(bay::HALF-0.055),-0.86,0.),h,0.018);c.entity(e).insert(MatingHinge);
        for z in [-0.6,0.6]{let e=rod(c,m,root,metal.clone(),h+Vec3::Z*(z-0.07),h+Vec3::Z*(z+0.07),0.023);c.entity(e).insert(MatingHinge);}
    }
    rigid(c,m,root,mesh(wall_v,wall_ix),coat.clone(),Transform::default());
    for slot in 0..6 {
        let p=bay::station(slot).as_vec3();
        block(c,m,root,metal.clone(),Vec3::new(p.x,bay::ROOF-0.035,p.z),Vec3::new(0.09,0.045,1.65));
        rod(c,m,root,coat.clone(),Vec3::new(p.x,bay::ROOF-0.075,p.z-0.23),Vec3::new(p.x,bay::ROOF-0.075,p.z+0.23),0.035);
        for z in [-0.65,0.65] {
            let p=bay::suspension(slot,0.,z);
            for arm in 0..2 {
                let a=p[arm];let b=p[arm+1];
                let e=part(c,m,root,Cuboid::new(0.018,1.,0.025).into(),metal.clone(),Transform::from_translation((a+b)*0.5).with_rotation(Quat::from_rotation_arc(Vec3::Y,(b-a).normalize())).with_scale(Vec3::new(1.,a.distance(b),1.)));
                c.entity(e).insert(Arm(slot,z,arm));
            }
            rod(c,m,root,seal.clone(),p[0]-Vec3::X*0.015,p[0]+Vec3::X*0.015,0.009);
            let e=part(c,m,root,Cylinder::new(0.009,0.03).mesh().resolution(12).build(),seal.clone(),Transform::from_translation(p[1]).with_rotation(Quat::from_rotation_arc(Vec3::Y,Vec3::X)));
            c.entity(e).insert(Elbow(slot,z));
        }
        let shoe=c.spawn((Transform::from_translation(p),Visibility::Inherited,ChildOf(root),Shoe(slot))).id();
        for z in [-0.65,0.65] {
            block(c,m,shoe,coat.clone(),Vec3::new(0.,0.10,z),Vec3::new(0.09,0.032,0.11));
            rod(c,m,shoe,seal.clone(),Vec3::new(-0.015,0.10,z),Vec3::new(0.015,0.10,z),0.009);
            for x in [-0.035,0.035] {rod(c,m,shoe,metal.clone(),Vec3::new(x,0.12,z),Vec3::new(x,0.078,z),0.008);}
        }
    }
}
pub fn update(f:Res<Flight>,mut leaves:Query<(&Leaf,&mut Transform)>,mut shoes:Query<(&Shoe,&mut Transform),Without<Leaf>>,mut arms:Query<(&Arm,&mut Transform),(Without<Leaf>,Without<Shoe>)>,mut elbows:Query<(&Elbow,&mut Transform),(Without<Leaf>,Without<Shoe>,Without<Arm>)>,mut last:Local<Option<(f64,[f64;6])>>) {
    if f.vehicle!=Vehicle::Gripen||f.aircraft!=Aircraft::F22{*last=None;return;}
    let key=(f.systems.bays,f.systems.main_stroke);if *last==Some(key){return;}*last=Some(key);
    for (leaf,mut t) in &mut leaves {t.set_if_neq(bay::door_pose(leaf.side,leaf.inner,f.systems.bays));}
    for (shoe,mut t) in &mut shoes {t.translation=bay::store_position(shoe.0,f.systems.main_stroke[shoe.0]).as_vec3();}
    for (arm,mut t) in &mut arms {
        let p=bay::suspension(arm.0,f.systems.main_stroke[arm.0],arm.1);let a=p[arm.2];let b=p[arm.2+1];
        t.set_if_neq(Transform::from_translation((a+b)*0.5).with_rotation(Quat::from_rotation_arc(Vec3::Y,(b-a).normalize())).with_scale(Vec3::new(1.,a.distance(b),1.)));
    }
    for (e,mut t) in &mut elbows {t.translation=bay::suspension(e.0,f.systems.main_stroke[e.0],e.1)[1];}
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::raptor_bay::tests::{Triangle,triangles,intersects,base_matrix,belongs,transformed};
    use std::collections::{HashMap,HashSet};
    fn leaf(w:&World,mut e:Entity)->Option<(usize,Mat4)> {
        loop {
            if let Some(l)=w.get::<Leaf>(e){return Some(((if l.side<0.{0}else{2})+usize::from(l.inner),base_matrix(w,e).inverse()));}
            let p=w.get::<ChildOf>(e)?;e=p.parent();
        }
    }
    fn cells(t:&Triangle)->Vec<(i32,i32,i32)> {
        let cell=|p:Vec3|((p.x/0.2).floor()as i32,(p.y/0.2).floor()as i32,(p.z/0.2).floor()as i32);
        let a=cell(t.min);let b=cell(t.max);let mut v=Vec::new();
        for x in a.0..=b.0{for y in a.1..=b.1{for z in a.2..=b.2{v.push((x,y,z));}}}v
    }
    #[test]
    fn assembled_main_doors_and_full_size_rounds_clear_their_complete_sweeps() {
        let mut app=App::new();app.init_resource::<Assets<Mesh>>().init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>().add_systems(Startup,super::super::setup);app.update();
        let w=app.world_mut();let root=w.query::<(Entity,&Root)>().iter(w).find(|(_,r)|r.0==Aircraft::F22).unwrap().0;
        let mut fixed=Vec::new();let mut groups:[Vec<Triangle>;4]=default();
        for (e,m,mat) in w.query::<(Entity,&Mesh3d,&MeshMaterial3d<StandardMaterial>)>().iter(w) {
            if !belongs(w,e,root){continue;}
            if w.resource::<Assets<StandardMaterial>>().get(&mat.0).unwrap().alpha_mode!=AlphaMode::Opaque{continue;}
            let mesh=w.resource::<Assets<Mesh>>().get(&m.0).unwrap();
            if let Some((i,inverse))=leaf(w,e) {
                groups[i].extend(triangles(mesh,inverse*base_matrix(w,e),e));continue;
            }
            if w.get::<Store>(e).is_some()||w.get::<MatingHinge>(e).is_some()||w.get::<Arm>(e).is_some()||w.get::<Elbow>(e).is_some(){continue;}
            fixed.extend(triangles(mesh,base_matrix(w,e),e).into_iter().filter(|t|t.max.y> -1.8&&t.min.y< -0.28&&t.max.x> -1.7&&t.min.x<1.7&&t.max.z>bay::FORE-0.2&&t.min.z<bay::AFT+0.2));
        }
        let missile=crate::combat::radar_visual::body(false,true);
        for slot in 0..6 {fixed.extend(triangles(&missile,Mat4::from_translation(bay::station(slot).as_vec3()),Entity::PLACEHOLDER));}
        let mut grid:HashMap<_,Vec<usize>>=HashMap::new();for(i,t)in fixed.iter().enumerate(){for c in cells(t){grid.entry(c).or_default().push(i);}}
        let mut failures=Vec::new();let mut checks=0;
        for k in 0..=100 {
            let p=k as f64/100.;
            for i in 0..4 {
                let side=if i<2{-1.}else{1.};let inner=i%2==1;
                for t in transformed(&groups[i],bay::door_pose(side,inner,p).to_matrix()) {
                    let c=t.p.into_iter().sum::<Vec3>()/3.;
                    // Journal/drive contacts within the 12 cm bearing envelope
                    // are intended mates; the door skins outside it are tested.
                    let hinge=bay::pivot(side,false);
                    if Vec2::new(c.x-hinge.x,c.y-hinge.y).length()<0.12{continue;}
                    let candidates=cells(&t).into_iter().flat_map(|c|grid.get(&c).into_iter().flatten().copied()).collect::<HashSet<_>>();
                    for j in candidates{checks+=1;if intersects(&t,&fixed[j]) {
                        if failures.len()<12{failures.push(format!("leaf {i} pose {k} at {c:?}: {:?} vs {:?} ({:?})",t.p,fixed[j].p,fixed[j].entity));}
                        break;
                    }}
                }
            }
        }
        assert!(checks>1000);assert!(failures.is_empty(),"{}",failures.join("\n"));
        // Actual detailed AIM-120 geometry, including fins, rather than spheres.
        let doors=(0..4).flat_map(|i|transformed(&groups[i],bay::door_pose(if i<2{-1.}else{1.},i%2==1,1.).to_matrix())).collect::<Vec<_>>();
        let mut door_grid:HashMap<_,Vec<usize>>=HashMap::new();for(i,t)in doors.iter().enumerate(){for c in cells(t){door_grid.entry(c).or_default().push(i);}}
        for slot in 0..6 {for k in 0..=100 {
            let pose=Mat4::from_translation(bay::store_position(slot,k as f64/100.).as_vec3());
            for t in triangles(&missile,pose,Entity::PLACEHOLDER) {
                let candidates=cells(&t).into_iter().flat_map(|c|door_grid.get(&c).into_iter().flatten().copied()).collect::<HashSet<_>>();
                for j in candidates {assert!(!intersects(&t,&doors[j]),"missile/door: slot {slot} stroke {k}");}
            }
        }}
        println!("Main bay: 404 leaf poses and 606 detailed missile ejection poses; {checks} broad-phase candidate checks");
    }
}
