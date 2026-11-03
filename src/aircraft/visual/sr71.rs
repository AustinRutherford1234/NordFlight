//! Original static SR-71A silhouette. No animated component tags or external assets.
use super::*;
fn body(sections:&[(f32,f32,f32,f32)],x:f32)->Mesh {
    let mut p=Vec::new();let mut ix=Vec::new();
    for (i,&(z,w,h,y)) in sections.iter().enumerate(){for j in 0..32 {
        let a=j as f32*std::f32::consts::TAU/32.;p.push(Vec3::new(x+w*a.cos(),y+h*a.sin(),z));
        if i>0 {let b=(i*32+j) as u32;let n=(i*32+(j+1)%32) as u32;ix.extend([b-32,n,b,n-32,n,b-32]);}
    }}mesh(p,ix)
}
fn plate(points:&[Vec3])->Mesh {
    let mut p=Vec::new();let mut ix=Vec::new();
    for dy in [-0.045,0.045] {p.extend(points.iter().map(|v|*v+Vec3::Y*dy));}
    let n=points.len() as u32;
    for i in 1..n-1 {ix.extend([0,i+1,i,n,n+i,n+i+1]);}
    for i in 0..n {let j=(i+1)%n;ix.extend([i,j,n+j,i,n+j,n+i]);}mesh(p,ix)
}
pub fn setup(c:&mut Commands,m:&mut Assets<Mesh>,mats:&mut Assets<StandardMaterial>,root:Entity){
    let skin=mats.add(StandardMaterial{base_color:Color::srgb(0.035,0.043,0.052),perceptual_roughness:0.68,metallic:0.25,cull_mode:None,..default()});
    let rim=material(mats,Color::srgb(0.11,0.12,0.13),0.6);
    let dark=material(mats,Color::srgb(0.006,0.008,0.01),0.1);
    let glass=mats.add(StandardMaterial{base_color:Color::srgb(0.075,0.15,0.19),metallic:0.65,perceptual_roughness:0.18,..default()});
    part(c,m,root,body(&[(-16.365,0.008,0.008,0.),(-14.,0.45,0.2,0.),(-11.,0.85,0.45,0.),(-8.,1.,0.65,0.),(-4.,1.1,0.7,0.),(2.,1.15,0.65,0.),(9.,1.,0.5,0.),(15.8,0.35,0.15,0.),(16.365,0.01,0.01,0.)],0.),skin.clone(),Transform::default());
    for side in [-1.,1.] {
        let v=|x,y,z|Vec3::new(side*x,y,z);
        part(c,m,root,plate(&[v(0.1,0.,-15.8),v(1.5,0.,-8.),v(3.2,0.,0.),v(8.47,0.,10.6),v(8.1,0.,13.),v(0.1,0.,14.)]),skin.clone(),Transform::default());
        let x=side*4.9;
        part(c,m,root,body(&[(-6.7,0.9,0.9,0.),(-5.8,1.18,1.18,0.),(-2.,1.3,1.3,0.),(5.,1.25,1.25,0.),(10.,1.03,1.03,0.),(12.4,0.96,0.96,0.)],x),skin.clone(),Transform::default());
        // Dark inlet disk behind a conical spike; visible annular intake, not a white cap.
        part(c,m,root,Cylinder::new(0.89,0.05).mesh().resolution(32).build(),dark.clone(),Transform::from_xyz(x,0.,-6.69).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)));
        part(c,m,root,body(&[(-9.2,0.005,0.005,0.),(-6.7,0.56,0.56,0.)],x),rim.clone(),Transform::default());
        part(c,m,root,body(&[(10.8,0.88,0.88,0.),(12.41,0.82,0.82,0.)],x),rim.clone(),Transform::default());
        part(c,m,root,Cylinder::new(0.8,0.04).mesh().resolution(32).build(),dark.clone(),Transform::from_xyz(x,0.,12.42).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)));
        part(c,m,root,plate(&[v(4.9,0.8,5.),v(4.25,4.05,8.7),v(4.15,3.9,11.5),v(4.9,0.8,12.)]),skin.clone(),Transform::default());
    }
    for z in [-9.,-6.9] {part(c,m,root,body(&[(z-1.,0.05,0.05,0.6),(z-0.5,0.55,0.48,0.72),(z+0.6,0.53,0.4,0.72),(z+1.,0.05,0.05,0.6)],0.),glass.clone(),Transform::default());}
    for (x,z,count) in [(0.,-9.5,2),(-2.4,3.,3),(2.4,3.,3)] {
        tube(c,m,root,rim.clone(),Vec3::new(x,-0.4,z),Vec3::new(x,-1.9,z),0.09);
        for i in 0..count {let offset=(i as f32-(count-1) as f32*0.5)*0.36;
            part(c,m,root,Torus::new(0.21,0.39).mesh().major_resolution(20).minor_resolution(8).build(),dark.clone(),Transform::from_xyz(x+offset,-1.9,z).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)));
        }
    }
}
#[cfg(test)]mod tests {use super::*;
#[test]fn static_mesh_has_finite_geometry(){let m=body(&[(-16.365,0.01,0.01,0.),(0.,1.,0.7,0.),(16.365,0.01,0.01,0.)],0.);let Some(bevy::mesh::VertexAttributeValues::Float32x3(p))=m.attribute(Mesh::ATTRIBUTE_POSITION)else{panic!()};assert!(p.iter().flatten().all(|v|v.is_finite()));assert!(m.indices().unwrap().iter().all(|i|i<p.len()));}
}
