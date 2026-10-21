//! Fitted Raptor cabin in cockpit-local coordinates. Display and picking
//! positions stay unchanged; structure stops behind their clear viewing faces.
use bevy::{asset::RenderAssetUsages, prelude::*, render::render_resource::PrimitiveTopology};

fn padded(size: Vec3) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut face = |a: Vec3, b: Vec3, c: Vec3| {
        let normal = (b-a).cross(c-a).normalize_or_zero();
        positions.extend([a.to_array(), b.to_array(), c.to_array()]);
        normals.extend([normal.to_array(); 3]);
    };
    let ring = |y: f32, inset: f32| {
        let x = size.x * 0.5 - inset;
        let z = size.z * 0.5 - inset;
        let r = 0.025_f32.min(x * 0.3).min(z * 0.3);
        [(-x+r,-z),(x-r,-z),(x,-z+r),(x,z-r),(x-r,z),(-x+r,z),(-x,z-r),(-x,-z+r)]
            .map(|(x,z)|Vec3::new(x,y,z))
    };
    let rings = [ring(-size.y*0.5,0.008), ring(size.y*0.2,0.), ring(size.y*0.5,0.012)];
    for j in 0..8 {
        let k = (j+1)%8;
        face(Vec3::Y*(size.y*0.5),rings[2][k],rings[2][j]);
        face(-Vec3::Y*(size.y*0.5),rings[0][j],rings[0][k]);
        for pair in rings.windows(2) {
            face(pair[0][j],pair[1][j],pair[1][k]);
            face(pair[0][j],pair[1][k],pair[0][k]);
        }
    }
    let mut mesh=Mesh::new(PrimitiveTopology::TriangleList,RenderAssetUsages::default());
    mesh.insert_indices(bevy::mesh::Indices::U32((0..positions.len() as u32).collect()));
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,vec![[0.,0.];positions.len()]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION,positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,normals);
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn padded_seat_batches_without_truncated_vertex_attributes() {
        let cushion=padded(Vec3::new(0.43,0.13,0.66));
        let mut mesh=Mesh::from(Cuboid::default());
        mesh.merge(&cushion).expect("compatible rigid seat geometry");
        let count=mesh.count_vertices();
        assert!(mesh.attributes().all(|(_,values)|values.len()==count));
        assert!(mesh.indices().unwrap().iter().all(|i|i<count));
    }
}

pub(super) fn setup(c:&mut Commands, meshes:&mut Assets<Mesh>, materials:&mut Assets<StandardMaterial>, root:Entity) {
    let mut material=|color:Color,metallic:f32|materials.add(StandardMaterial {
        base_color:color,metallic,perceptual_roughness:0.86,..default()
    });
    let tub=material(Color::srgb(0.026,0.031,0.035),0.05);
    let frame=material(Color::srgb(0.065,0.074,0.078),0.45);
    let fabric=material(Color::srgb(0.22,0.25,0.17),0.);
    let belt=material(Color::srgb(0.48,0.46,0.33),0.);
    let silver=material(Color::srgb(0.36,0.38,0.37),0.7);
    let yellow=material(Color::srgb(0.88,0.64,0.07),0.);
    let cube=meshes.add(Cuboid::default());
    let cylinder=meshes.add(Cylinder::new(1.,1.).mesh().resolution(12).build());
    let spawn=|c:&mut Commands, mesh:Handle<Mesh>, mat:Handle<StandardMaterial>, at:Vec3, scale:Vec3, rotation:Quat| {
        let e=c.spawn((Mesh3d(mesh),MeshMaterial3d(mat),
            Transform::from_translation(at).with_scale(scale).with_rotation(rotation),
            crate::render_batch::RigidGeometry)).id();
        c.entity(root).add_child(e);
    };
    let block=|c:&mut Commands, mat:Handle<StandardMaterial>,at:Vec3,size:Vec3,rotation:Quat| {
        spawn(c,cube.clone(),mat,at,size,rotation);
    };
    let rod=|c:&mut Commands,mat:Handle<StandardMaterial>,a:Vec3,b:Vec3,r:f32| {
        let d=b-a;
        spawn(c,cylinder.clone(),mat,(a+b)*0.5,Vec3::new(r,d.length(),r),
            Quat::from_rotation_arc(Vec3::Y,d.normalize()));
    };
    // Matte non-slip footwell. Raised ribs stay below the pedals, not over
    // the MFDs; sidewall/console forward ends stop aft of the radar face.
    block(c,tub.clone(),Vec3::new(0.,-0.15,-2.64),Vec3::new(0.96,0.08,2.42),Quat::IDENTITY);
    // Raised forward footwell clears the recessed nose-gear strut pocket. The
    // seat floor remains lower, with an opaque kick panel closing the step.
    block(c,tub.clone(),Vec3::new(0.,0.26,-3.43),Vec3::new(0.94,0.08,0.86),Quat::IDENTITY);
    block(c,tub.clone(),Vec3::new(0.,0.055,-3.01),Vec3::new(0.94,0.41,0.035),Quat::IDENTITY);
    for side in [-1.,1.] {
        block(c,tub.clone(),Vec3::new(side*0.49,0.15,-2.39),Vec3::new(0.04,0.56,1.82),Quat::IDENTITY);
        block(c,frame.clone(),Vec3::new(side*0.38,0.35,-2.67),Vec3::new(0.22,0.14,1.24),Quat::IDENTITY);
        block(c,tub.clone(),Vec3::new(side*0.38,0.427,-2.67),Vec3::new(0.23,0.014,1.25),Quat::IDENTITY);
        for row in 0..9 {
            block(c,frame.clone(),Vec3::new(side*0.18,0.303,-3.76+row as f32*0.075),
                Vec3::new(0.21,0.006,0.008),Quat::IDENTITY);
        }
        // Seat guide rails, under-seat support and restraint housings.
        rod(c,silver.clone(),Vec3::new(side*0.22,-0.08,-1.84),Vec3::new(side*0.22,0.87,-1.69),0.016);
        block(c,frame.clone(),Vec3::new(side*0.21,0.055,-2.30),Vec3::new(0.075,0.26,0.79),Quat::IDENTITY);
        block(c,frame.clone(),Vec3::new(side*0.255,0.25,-2.18),Vec3::new(0.055,0.28,0.22),Quat::IDENTITY);
    }
    block(c,tub.clone(),Vec3::new(0.,0.19,-1.43),Vec3::new(0.96,0.68,0.06),Quat::IDENTITY);
    block(c,frame.clone(),Vec3::new(0.,0.21,-2.51),Vec3::new(0.48,0.15,0.72),Quat::IDENTITY);
    // Chamfered upholstered survival-pack pan and separately reclined back.
    let pan=meshes.add(padded(Vec3::new(0.43,0.13,0.66)));
    spawn(c,pan,fabric.clone(),Vec3::new(0.,0.30,-2.57),Vec3::ONE,Quat::from_rotation_x(-0.035));
    let back=meshes.add(padded(Vec3::new(0.43,0.11,0.64)));
    spawn(c,back,fabric.clone(),Vec3::new(0.,0.53,-2.08),Vec3::ONE,
        Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2+0.16));
    let head=meshes.add(padded(Vec3::new(0.33,0.15,0.27)));
    spawn(c,head,tub.clone(),Vec3::new(0.,0.91,-1.99),Vec3::ONE,
        Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2+0.16));
    for side in [-1.,1.] {
        let bolster=meshes.add(padded(Vec3::new(0.055,0.15,0.56)));
        spawn(c,bolster,fabric.clone(),Vec3::new(side*0.22,0.36,-2.55),Vec3::ONE,Quat::IDENTITY);
        // Shoulder webbing follows the seat rake and meets a lap buckle.
        block(c,belt.clone(),Vec3::new(side*0.105,0.59,-2.141),Vec3::new(0.048,0.44,0.012),
            Quat::from_rotation_x(0.16)*Quat::from_rotation_z(side*0.12));
        block(c,belt.clone(),Vec3::new(side*0.12,0.377,-2.48),Vec3::new(0.23,0.012,0.048),
            Quat::from_rotation_y(side*0.25));
        block(c,silver.clone(),Vec3::new(side*0.15,0.38,-2.49),Vec3::new(0.044,0.018,0.057),Quat::IDENTITY);
        // Stitched channels in the pan, with low contrast at normal distance.
        for z in [-2.77,-2.64,-2.51] {
            block(c,belt.clone(),Vec3::new(side*0.18,0.369,z),Vec3::new(0.002,0.001,0.075),Quat::IDENTITY);
        }
    }
    block(c,frame.clone(),Vec3::new(0.,0.39,-2.49),Vec3::new(0.065,0.022,0.065),Quat::IDENTITY);
    // Visible centre pull handle; alternating sleeves suggest warning striping.
    for j in 0..16 {
        let at=|j:usize| {
            let t=j as f32/16.*std::f32::consts::TAU;
            Vec3::new(0.055*t.cos(),0.40+0.045*t.sin(),-2.91)
        };
        rod(c,if j%2==0{yellow.clone()}else{tub.clone()},at(j),at(j+1),0.007);
    }
}
