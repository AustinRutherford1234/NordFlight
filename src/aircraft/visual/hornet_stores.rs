//! Shared attached/released tank mesh and world-space dropped-store presentation.
use super::*;
use crate::aircraft::hornet::stores as tanks;
#[derive(Component)]
pub(crate) struct HornetRail(pub(crate) usize);
#[derive(Component)]
pub(crate) struct TankVisual(usize, bool);
// Fitted to the imported inlet underside at the two fuselage stations.
// The short swept adapter joins the rail to the skin without moving the store.
const SHOULDER_PROFILE:[(f32,f32);5]=[(-0.30,0.18),(-0.19,0.12),(-0.09,0.065),(0.01,0.015),(0.12,-0.035)];
fn shoulder_adapter(station:Vec3)->Mesh {
    let mut p=Vec::<[f32;3]>::new();let mut n=Vec::<[f32;3]>::new();
    let mut face=|a:Vec3,b:Vec3,c:Vec3|{let normal=(c-a).cross(b-a).normalize_or_zero().to_array();p.extend([a.to_array(),c.to_array(),b.to_array()]);n.extend([normal;3]);};
    let point=|z:f32,y:f32,x:f32|Vec3::new(x,y-station.y,z-station.z);
    for pair in SHOULDER_PROFILE.windows(2) {
        let (z0,y0)=pair[0];let(z1,y1)=pair[1];
        let a=point(z0,-0.34,-0.045);let b=point(z0,-0.34,0.045);let c=point(z0,y0,0.045);let d=point(z0,y0,-0.045);
        let e=point(z1,-0.34,-0.045);let f=point(z1,-0.34,0.045);let g=point(z1,y1,0.045);let h=point(z1,y1,-0.045);
        for q in [[a,e,f,b],[d,c,g,h],[a,d,h,e],[b,f,g,c]] {face(q[0],q[1],q[2]);face(q[0],q[2],q[3]);}
        if z0==SHOULDER_PROFILE[0].0 {face(a,b,c);face(a,c,d);}
        if z1==SHOULDER_PROFILE[4].0 {face(e,h,g);face(e,g,f);}
    }
    // Extend the existing short launcher aft beneath the adapter foot.
    let a=point(-0.35,-0.44,-0.05);let b=point(-0.35,-0.44,0.05);let c=point(-0.35,-0.32,0.05);let d=point(-0.35,-0.32,-0.05);
    let e=point(0.25,-0.44,-0.05);let f=point(0.25,-0.44,0.05);let g=point(0.25,-0.32,0.05);let h=point(0.25,-0.32,-0.05);
    for q in [[a,e,f,b],[d,c,g,h],[a,d,h,e],[b,f,g,c],[a,b,c,d],[e,h,g,f]] {face(q[0],q[1],q[2]);face(q[0],q[2],q[3]);}
    let mut mesh=Mesh::new(PrimitiveTopology::TriangleList,RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION,p);mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,n);mesh
}
pub(super) fn shoulder_mounts(c:&mut Commands,meshes:&mut Assets<Mesh>,mats:&mut Assets<StandardMaterial>,root:Entity) {
    let paint=material(mats,Color::srgb(0.42,0.45,0.46),0.3);
    let hardware=material(mats,Color::srgb(0.20,0.23,0.24),0.6);
    for i in [6,7] {
        let at=crate::aircraft::station(Aircraft::Hornet,i).as_vec3();
        let e=part(c,meshes,root,shoulder_adapter(at),paint.clone(),Transform::from_translation(at));
        c.entity(e).insert(Name::new(format!("HornetShoulderAdapter{i}")));
        box_part(c,meshes,root,paint.clone(),Vec3::new(at.x,-0.325,-0.1),Vec3::new(0.13,0.045,0.55));
        for z in [-0.22,0.02] { for side in [-1.,1.] {
            part(c,meshes,root,Cylinder::new(0.018,0.012).mesh().resolution(8).build(),hardware.clone(),Transform::from_xyz(at.x+side*0.05,-0.20,z).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)));
        }}
    }
}
#[cfg(test)]mod shoulder_tests {
    use super::*;
    #[test]fn shoulder_mount_and_actual_missile_mesh_export_for_sweep_validation() {
        let missile=crate::combat::radar_visual::body(false,true);
        let mut result=Vec::new();
        for i in [6,7] {let at=crate::aircraft::station(Aircraft::Hornet,i).as_vec3();
            let adapter=shoulder_adapter(at);
            let geometry=|mesh:&Mesh| {let bevy::mesh::VertexAttributeValues::Float32x3(v)=mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()else{panic!()};let ix:Vec<usize>=mesh.indices().map(|i|i.iter().collect()).unwrap_or_else(||(0..v.len()).collect());serde_json::json!({"vertices":v,"indices":ix})};
            result.push(serde_json::json!({"station":at.to_array(),"missile":geometry(&missile),"adapter":geometry(&adapter)}));
            assert!((at.z+0.85).abs()<1e-6);
            assert!(SHOULDER_PROFILE.iter().all(|(_,y)|*y> -0.32));
        }
        std::fs::create_dir_all("tests/shoulder-mount-review").unwrap();
        std::fs::write("tests/shoulder-mount-review/geometry.json",serde_json::to_vec(&result).unwrap()).unwrap();
    }
}
pub(super) fn tank_mesh() -> Mesh {
    let profile: [(f32, f32); 13] = PROFILE;
    let radial_scale = tanks::TANK_RADIUS as f32 / 0.375;
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uv = Vec::new();
    let mut index = Vec::new();
    const N: usize = 48;
    for (i, (z, r)) in profile.iter().enumerate() {
        let r = *r * radial_scale;
        let a = profile[i.saturating_sub(1)];
        let b = profile[(i + 1).min(profile.len() - 1)];
        let slope = (b.1 - a.1) * radial_scale / (b.0 - a.0);
        for j in 0..=N {
            let t = j as f32 * std::f32::consts::TAU / N as f32;
            vertices.push([r * t.cos(), r * t.sin(), *z]);
            normals.push(Vec3::new(t.cos(), t.sin(), -slope).normalize().to_array());
            uv.push([j as f32 / N as f32, i as f32 / (profile.len() - 1) as f32]);
            if i > 0 && j > 0 {
                let k = (i * (N + 1) + j) as u32;
                let prev = k - (N + 1) as u32;
                index.extend_from_slice(&[prev - 1, k, k - 1, prev - 1, prev, k]);
            }
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_indices(Indices::U32(index));
    mesh
}
const PROFILE: [(f32, f32); 13] = [
    (-2.40, 0.005),
    (-2.30, 0.09),
    (-2.05, 0.22),
    (-1.65, 0.32),
    (-1.10, 0.365),
    (-0.4, 0.375),
    (0.4, 0.375),
    (1.1, 0.35),
    (1.55, 0.28),
    (1.85, 0.18),
    (2.10, 0.10),
    (2.28, 0.04),
    (2.35, 0.005),
];
pub(super) fn setup(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    root: Entity,
) {
    let body = meshes.add(tank_mesh());
    let paint = material(mats, Color::srgb(0.43, 0.47, 0.49), 0.25);
    let seam = material(mats, Color::srgb(0.23, 0.26, 0.27), 0.4);
    box_part(
        c,
        meshes,
        root,
        seam.clone(),
        Vec3::new(0., -0.45, tanks::STATIONS[2].z as f32),
        Vec3::new(0.14, 0.60, 1.2),
    );
    for i in 0..3 {
        for dropped in [false, true] {
            let parent = c
                .spawn((
                    TankVisual(i, dropped),
                    Transform::from_translation(tanks::STATIONS[i].as_vec3()),
                    Visibility::Hidden,
                    Name::new(format!(
                        "HornetTank{i}-{}",
                        if dropped { "released" } else { "attached" }
                    )),
                ))
                .id();
            if !dropped {
                c.entity(root).add_child(parent);
            }
            let hull = c
                .spawn((
                    Mesh3d(body.clone()),
                    MeshMaterial3d(paint.clone()),
                    Transform::default(),
                ))
                .id();
            c.entity(parent).add_child(hull);
            for (z, r) in [(-1.1f32, 0.365f32), (0., 0.375), (1.1, 0.35)] {
                let ring = part(
                    c,
                    meshes,
                    parent,
                    Torus::new(
                        r * (tanks::TANK_RADIUS as f32 / 0.375) - 0.003,
                        r * (tanks::TANK_RADIUS as f32 / 0.375) + 0.003,
                    )
                    .mesh()
                    .major_resolution(48)
                    .minor_resolution(6)
                    .build(),
                    seam.clone(),
                    Transform::from_xyz(0., 0., z)
                        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                );
                c.entity(ring).insert(Name::new("Tank seam"));
            }
            // Three shallow rear stabilizing fins and a top mounting saddle.
            for a in [0f32, 120., 240.] {
                let q = Quat::from_rotation_z(a.to_radians());
                let p = q * Vec3::new(0., 0.27, 1.65);
                let fin = box_part(
                    c,
                    meshes,
                    parent,
                    paint.clone(),
                    p,
                    Vec3::new(0.018, 0.30, 0.65),
                );
                c.entity(fin)
                    .insert(Transform::from_translation(p).with_rotation(q));
            }
            box_part(
                c,
                meshes,
                parent,
                seam.clone(),
                Vec3::new(0., 0.45, -0.1),
                Vec3::new(0.12, 0.08, 1.0),
            );
        }
    }
}
pub(crate) fn update(
    f: Res<Flight>,
    mut nodes: Query<(&TankVisual, &mut Transform, &mut Visibility)>,
    mut rails: Query<(&HornetRail, &mut Transform), Without<TankVisual>>,
) {
    if f.aircraft == Aircraft::Hornet {
        for (tag, mut transform) in &mut rails {
            let q = crate::aircraft::store_rotation(&f, tag.0);
            transform.translation = (crate::aircraft::station_for(&f, tag.0)
                + q * bevy::math::DVec3::Y * 0.14)
                .as_vec3();
            transform.rotation = q.as_quat();
        }
    }
    for (tag, mut transform, mut visibility) in &mut nodes {
        let mut show = false;
        if f.aircraft == Aircraft::Hornet {
            if tag.1 {
                if let Some(d) = f.hornet.tanks.dropped[tag.0].as_ref().filter(|d| d.alive) {
                    show = true;
                    transform.translation = crate::world::local(d.position, &f);
                    transform.rotation = crate::world::local_rot(d.orientation, &f);
                }
            } else {
                show = f.hornet.tanks.fuel[tag.0].is_some();
            }
        }
        visibility.set_if_neq(if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tank_body_has_room_for_480_gallons_and_ullage() {
        let scale = tanks::TANK_RADIUS / 0.375;
        let volume = PROFILE
            .windows(2)
            .map(|p| {
                let a = p[0].1 as f64 * scale;
                let b = p[1].1 as f64 * scale;
                std::f64::consts::PI * (p[1].0 - p[0].0) as f64 * (a * a + a * b + b * b) / 3.
            })
            .sum::<f64>();
        assert!(volume > 480. * 0.003785411784 * 1.05);
    }
    #[test]
    fn tank_has_bounded_finite_geometry() {
        let mesh = tank_mesh();
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!()
        };
        assert!(p.len() > 600);
        assert!(p.iter().all(|p| p.iter().all(|v| v.is_finite())
            && p[0].abs() <= 0.433
            && p[1].abs() <= 0.433
            && p[2].abs() <= 2.401));
    }
}
