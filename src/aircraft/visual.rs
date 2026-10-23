//! Bounded code-native fleet meshes. Geometry and collision share planforms.
use super::*;
use bevy::{asset::RenderAssetUsages, mesh::Indices, render::render_resource::PrimitiveTopology};
mod raptor_mesh;
mod raptor_intake;
mod raptor_finish;
pub mod raptor_bay;
pub mod raptor_main_bay;
pub mod raptor_detail;
#[derive(Component)]
pub struct Root(pub Aircraft);
#[derive(Component)]
pub struct AircraftPart(pub Aircraft);
#[derive(Component)]
pub struct CockpitRoot(pub Aircraft);
#[cfg(test)]
mod cabin_clearance_tests {
    use super::*;
    fn matrix(w:&World,e:Entity)->Mat4 {
        let m=w.get::<Transform>(e).map_or(Mat4::IDENTITY,Transform::to_matrix);
        w.get::<ChildOf>(e).map_or(m,|p|matrix(w,p.parent())*m)
    }
    fn belongs(w:&World,mut e:Entity,root:Entity)->bool {
        loop { if e==root{return true;} let Some(p)=w.get::<ChildOf>(e) else{return false;};e=p.parent(); }
    }
    #[test]
    fn cockpit_floor_and_seat_are_not_occluded_by_airframe_or_gear_well() {
        let mut app=App::new();
        app.init_resource::<Assets<Mesh>>().init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>().add_systems(Startup,setup);
        app.update();
        let w=app.world_mut();
        let root=w.query::<(Entity,&Root)>().iter(w).find(|(_,r)|r.0==Aircraft::F22).unwrap().0;
        let mut hits=Vec::new();
        for (x,z) in [(0.,-3.3_f32),(0.,-2.8),(0.,-2.4),(0.29,-3.44),(-0.29,-3.44)] {
            let point=Vec2::new(x,Aircraft::F22.cabin_offset().z+z);
            for (e,handle,material) in w.query::<(Entity,&Mesh3d,&MeshMaterial3d<StandardMaterial>)>().iter(w) {
                if !belongs(w,e,root) {continue;}
                let mesh=w.resource::<Assets<Mesh>>().get(&handle.0).unwrap();
                let mat=w.resource::<Assets<StandardMaterial>>().get(&material.0).unwrap();
                if mat.alpha_mode!=AlphaMode::Opaque {continue;}
                let Some(bevy::mesh::VertexAttributeValues::Float32x3(v))=mesh.attribute(Mesh::ATTRIBUTE_POSITION) else{continue;};
                let t=matrix(w,e);
                let indices:Vec<_>=mesh.indices().map_or_else(||(0..v.len()).collect(),|i|i.iter().collect());
                for tri in indices.chunks_exact(3) {
                    let p=[tri[0],tri[1],tri[2]].map(|i|t.transform_point3(Vec3::from_array(v[i])));
                    let q=p.map(|p|Vec2::new(p.x,p.z));
                    let det=(q[1]-q[0]).perp_dot(q[2]-q[0]);
                    if det.abs()<1e-8 {continue;}
                    let u=(point-q[0]).perp_dot(q[2]-q[0])/det;
                    let v=(q[1]-q[0]).perp_dot(point-q[0])/det;
                    if u>=-1e-5&&v>=-1e-5&&u+v<=1.00001 {
                        let y=p[0].y+(p[1].y-p[0].y)*u+(p[2].y-p[0].y)*v;
                        hits.push((x,z,y,e,mat.base_color));
                    }
                }
            }
        }
        for (x,z) in [(0.,-3.3_f32),(0.,-2.8),(0.,-2.4),(0.29,-3.44),(-0.29,-3.44)] {
            let top=hits.iter().filter(|h|h.0==x&&h.1==z).max_by(|a,b|a.2.total_cmp(&b.2)).expect("closed floor or seat");
            assert!(top.2 > -0.3 && top.2 < 0.4,"cabin obstruction: {top:?}");
            assert!(top.4.to_srgba().red<0.3,"airframe/white wheel-well occludes interior: {top:?}");
        }
    }
}
#[derive(Component)]
pub(crate) struct Moving(Aircraft, usize);
#[derive(Component)]
pub(crate) struct Leg(Aircraft, usize);
#[derive(Component)]
pub(crate) struct Tire(Aircraft, usize);
#[derive(Component)]
pub(crate) struct Door(Aircraft, usize, f32, Vec3);
#[derive(Component)]
pub(crate) struct Nozzle(Aircraft, usize);
#[derive(Component)]
pub(crate) struct Petal(Aircraft, usize, usize);
#[derive(Component)]
pub(crate) struct Plume(Aircraft, usize, usize);
#[derive(Component)]
pub(crate) struct LiftDoor(Aircraft, f32, Vec3);
#[derive(Component)]
pub(crate) struct Lamp(Aircraft, u8);
#[derive(Component)]
pub(crate) struct Scar(Aircraft, usize);
#[derive(Component)]
pub(crate) struct Store(Aircraft, usize);
#[derive(Component)]
pub(crate) struct WeaponDoor(Aircraft, f32, Vec3);
#[derive(Component)]
pub(crate) struct Gauge(Aircraft, u8);
#[derive(Component)]
pub(crate) struct FanSpin(Aircraft);
#[derive(Component)]
pub(crate) struct Lens(Aircraft);

pub fn sections(a: Aircraft) -> Vec<(f64, f64, f64, f64)> {
    if a==Aircraft::F22{return super::raptor::sections();}
    let l = a.spec().length;
    let width = match a {
        Aircraft::Su27 => 1.10,
        Aircraft::F22 => 1.90,
        Aircraft::F35B => 1.25,
        _ => 1.05,
    };
    [
        (-0.50, 0.015, 0.02, 0.),
        (-0.47, 0.22, 0.20, 0.),
        (-0.41, 0.46, 0.36, 0.),
        (-0.35, 0.66, 0.47, 0.),
        (-0.29, 0.73, 0.53, 0.),
        (-0.24, 0.77, 0.56, 0.),
        (-0.18, 0.90, 0.58, 0.),
        (-0.10, width, 0.60, 0.),
        (0., width, 0.65, 0.),
        (0.13, width, 0.67, 0.),
        (0.26, width * 0.90, 0.59, 0.),
        (0.36, width * 0.78, 0.48, 0.),
        (0.42, width * 0.62, 0.38, 0.),
        (0.46, width * 0.30, 0.23, 0.),
    ]
    .map(|(z, w, h, y)| {
        (
            z * l,
            w * if matches!(a, Aircraft::F22 | Aircraft::F35B) && z < -0.15 && z > -0.34 {
                1.15
            } else {
                1.
            },
            h * if a == Aircraft::F35B { 1.18 } else { 1. },
            y,
        )
    })
    .to_vec()
}
pub fn wing(a: Aircraft, side: f32) -> [Vec3; 4] {
    if a==Aircraft::F22{return super::raptor::wing(side);}
    let s = a.spec();
    let half = s.span as f32 / 2.;
    let l = s.length as f32;
    let (sweep, tip_z) = match a {
        Aircraft::F22 => (-0.17, 0.06),
        Aircraft::Su27 => (-0.13, 0.14),
        Aircraft::F35B => (-0.12, 0.07),
        _ => (-0.13, 0.08),
    };
    [
        Vec3::new(side * 0.9, -0.08, l * sweep),
        Vec3::new(side * half, -0.08, l * tip_z),
        Vec3::new(side * (half - 0.22), -0.08, l * (tip_z + 0.075)),
        Vec3::new(side * 1.1, -0.08, l * 0.18),
    ]
}
pub fn panel_polygon(p: Panel) -> ([Vec3; 4], Vec3) {
    if let Some(outline)=p.outline{return (outline.map(|v|v.as_vec3()),p.normal.as_vec3()*0.035);}
    let span = (p.area / p.chord.max(0.01)) as f32;
    let c = p.chord as f32;
    let at = p.center.as_vec3();
    if p.normal.x.abs() > 0.5 {
        let up = p.axis().as_vec3();
        (
            [
                at - up * span * 0.5 - Vec3::Z * c * 0.35,
                at + up * span * 0.5 + Vec3::Z * c * 0.3,
                at + up * span * 0.45 + Vec3::Z * c * 0.95,
                at - up * span * 0.5 + Vec3::Z * c * 0.8,
            ],
            p.normal.as_vec3() * 0.05,
        )
    } else {
        (
            [
                at + Vec3::new(-span * 0.5, 0., -c * 0.15),
                at + Vec3::new(span * 0.5, 0., c * 0.15),
                at + Vec3::new(span * 0.44, 0., c * 0.95),
                at + Vec3::new(-span * 0.52, 0., c * 0.85),
            ],
            Vec3::Y * 0.045,
        )
    }
}
pub fn fin(a: Aircraft, side: f32) -> ([Vec3; 4], Vec3) {
    if a==Aircraft::F22 {
        return ([Vec3::new(side*1.56,0.36,2.75),Vec3::new(side*2.97,2.92,4.36),Vec3::new(side*2.97,2.92,4.95),Vec3::new(side*1.60,0.44,5.35)],Vec3::new(0.045,-side*0.025,0.));
    }
    let p = a.panels()[if side < 0. { 3 } else { 7 }];
    let (r, thick) = panel_polygon(p);
    (
        [r[0] - Vec3::Z * 1.0, r[1] - Vec3::Z * 0.55, r[1], r[0]],
        thick,
    )
}
/// Subtract actual moving-panel planforms from the fixed wing. Each output is
/// convex and is shared by rendering and collision; detachment leaves a hole.
pub fn fixed_wing(a: Aircraft, side: f32) -> Vec<Vec<Vec3>> {
    fixed_wing_with(a, side, wing(a, side).to_vec(), a.panels())
}
fn fixed_wing_with(_a: Aircraft, side: f32, outline: Vec<Vec3>, panels: [Panel;SURFACES]) -> Vec<Vec<Vec3>> {
    fn clip(points: &[Vec3], distance: impl Fn(Vec3) -> f32) -> Vec<Vec3> {
        let mut out = Vec::new();
        if points.is_empty() {
            return out;
        }
        let mut prev = *points.last().unwrap();
        let mut dp = distance(prev);
        for &p in points {
            let d = distance(p);
            if (d >= 0.) != (dp >= 0.) {
                out.push(prev + (p - prev) * (dp / (dp - d)));
            }
            if d >= 0. {
                out.push(p);
            }
            prev = p;
            dp = d;
        }
        out
    }
    let mut pieces = vec![outline];
    for (i, p) in panels.into_iter().enumerate() {
        if ![1, 2, 5, 6, 9, 10].contains(&i) || p.center.x.signum() as f32 != side {
            continue;
        }
        let (cut, _) = panel_polygon(p);
        let area = (0..4)
            .map(|i| cut[i].x * cut[(i + 1) % 4].z - cut[(i + 1) % 4].x * cut[i].z)
            .sum::<f32>();
        let sign = area.signum();
        let mut next = Vec::new();
        for piece in pieces {
            let mut inside = piece;
            for k in 0..4 {
                if inside.len() < 3 {
                    break;
                }
                let at = cut[k];
                let edge = cut[(k + 1) % 4] - at;
                let distance = |v: Vec3| (edge.x * (v.z - at.z) - edge.z * (v.x - at.x)) * sign;
                let outside = clip(&inside, |v| -distance(v));
                if outside.len() >= 3 {
                    next.push(outside);
                }
                inside = clip(&inside, distance);
            }
        }
        pieces = next;
    }
    pieces
        .into_iter()
        .filter_map(|mut p| {
            let mut i = 0;
            while p.len() >= 3 && i < p.len() {
                let prev = p[(i + p.len() - 1) % p.len()];
                let next = p[(i + 1) % p.len()];
                if (p[i] - prev).cross(next - p[i]).length_squared() < 1e-12 {
                    p.remove(i);
                    i = 0;
                } else {
                    i += 1;
                }
            }
            (p.len() >= 3).then_some(p)
        })
        .collect()
}
fn mesh(p: Vec<Vec3>, idx: Vec<u32>) -> Mesh {
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    m.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        p.iter().map(|p| p.to_array()).collect::<Vec<_>>(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; p.len()]);
    m.insert_indices(Indices::U32(idx));
    m.compute_normals();
    m
}
fn quad(v: &mut Vec<Vec3>, i: &mut Vec<u32>, q: [Vec3; 4]) {
    let n = v.len() as u32;
    v.extend(q);
    i.extend([n, n + 1, n + 2, n, n + 2, n + 3]);
}
fn solid(p: &[Vec3], thickness: Vec3) -> Mesh {
    let mut ordered = p.to_vec();
    if (p[1] - p[0]).cross(p[2] - p[0]).dot(thickness) < 0. {
        ordered.reverse();
    }
    let p = ordered.as_slice();
    let mut v = Vec::new();
    let mut idx = Vec::new();
    for k in 1..p.len() - 1 {
        let n = v.len() as u32;
        v.extend([
            p[0] + thickness,
            p[k] + thickness,
            p[k + 1] + thickness,
            p[0] - thickness,
            p[k + 1] - thickness,
            p[k] - thickness,
        ]);
        idx.extend(n..n + 6);
    }
    for k in 0..p.len() {
        let j = (k + 1) % p.len();
        quad(
            &mut v,
            &mut idx,
            [
                p[k] - thickness,
                p[j] - thickness,
                p[j] + thickness,
                p[k] + thickness,
            ],
        );
    }
    mesh(v, idx)
}
fn hull_mesh(a: Aircraft, radome: bool) -> Mesh {
    let mut sec = sections(a);
    if a == Aircraft::F22 {
        sec=raptor_intake::body_sections();
    }
    let mut v = Vec::new();
    let mut idx = Vec::new();
    let point = |(z, w, h, y): (f64, f64, f64, f64), k: usize| {
        let t = k as f32 / 64. * std::f32::consts::TAU;
        if a==Aircraft::F22 {
            return raptor_intake::body_point((z,w,h,y),k);
        }
        Vec3::new(
            w as f32
                * if matches!(a, Aircraft::F22 | Aircraft::F35B) {
                    t.cos().signum() * t.cos().abs().powf(0.6)
                } else {
                    t.cos()
                },
            y as f32
                + h as f32
                    * if matches!(a, Aircraft::F22 | Aircraft::F35B) {
                        t.sin().signum() * t.sin().abs().powf(1.6)
                    } else {
                        t.sin()
                    },
            z as f32,
        )
    };
    for pair in sec.windows(2) {
        let z = (pair[0].0 + pair[1].0) * 0.5;
        if (z < -a.spec().length * 0.34) != radome {
            continue;
        }
        for k in 0..64 {
            let q = [
                point(pair[0], k),
                point(pair[0], k + 1),
                point(pair[1], k + 1),
                point(pair[1], k),
            ];
            // Loft quads are not planar. Clip their actual render triangles so
            // an inlet/body intersection cannot re-triangulate across the seam.
            if a == Aircraft::F22 {
                for triangle in [[q[0],q[1],q[2]],[q[0],q[2],q[3]]] {
                    for polygon in cut_skin(a,triangle.to_vec()) {polygon_into(&mut v,&mut idx,&polygon);}
                }
            } else {
                for polygon in cut_skin(a, q.to_vec()) {polygon_into(&mut v, &mut idx, &polygon);}
            }
        }
    }
    if radome {
        let z = sec[0].0 as f32;
        for k in 0..64 {
            let n = v.len() as u32;
            v.extend([Vec3::new(0., 0., z), point(sec[0], k + 1), point(sec[0], k)]);
            idx.extend(n..n + 3);
        }
    } else {
        let last = *sec.last().unwrap();
        for k in 0..64 {
            let cap=[
                Vec3::new(0., 0., last.0 as f32),
                point(last, k),
                point(last, k + 1),
            ];
            for polygon in cut_skin(a,cap.to_vec()){polygon_into(&mut v,&mut idx,&polygon);}
        }
    }
    let m=mesh(v,idx);finish(a,if a==Aircraft::F22{raptor_intake::seam_normals(raptor_mesh::smooth(m))}else{m})
}
fn finish(a:Aircraft,m:Mesh)->Mesh{if a==Aircraft::F22{raptor_mesh::paint(m)}else{m}}
fn polygon_into(v: &mut Vec<Vec3>, indices: &mut Vec<u32>, p: &[Vec3]) {
    if p.len() < 3 {
        return;
    }
    for i in 1..p.len() - 1 {
        if (p[i] - p[0]).cross(p[i + 1] - p[0]).length_squared() < 1e-12 {
            continue;
        }
        let n = v.len() as u32;
        v.extend([p[0], p[i], p[i + 1]]);
        indices.extend(n..n + 3);
    }
}
fn cut_skin(a: Aircraft, points: Vec<Vec3>) -> Vec<Vec<Vec3>> {
    let pieces=cut_skin_access(a,vec![points]);
    if a==Aircraft::F22{raptor_intake::cut_openings(pieces)}else{pieces}
}
fn cut_skin_access(a:Aircraft, points:Vec<Vec<Vec3>>)->Vec<Vec<Vec3>> {
    let d = a.cabin_offset();
    let mut cuts = if a==Aircraft::F22{Vec::new()}else{vec![(
        Vec3::new(-0.55, 0.25, d.z - 4.05),
        Vec3::new(0.55, 4., d.z - 1.4),
    )]};
    for i in 0..3 {
        let at = gear_mount(a, i).as_vec3();
        let (at, half_width) = if a == Aircraft::F22 { raptor_detail::well_layout(i) } else { (at, 0.42) };
        cuts.push((
            Vec3::new(at.x - half_width, -4., at.z - 1.85),
            Vec3::new(at.x + half_width, -0.22, at.z + 0.65),
        ));
    }
    if a == Aircraft::F35B {
        cuts.push((Vec3::new(-0.95, -4., -1.75), Vec3::new(0.95, -0.28, 1.75)));
    }
    if a == Aircraft::F35B {
        cuts.push((Vec3::new(-0.85, 0.22, -3.45), Vec3::new(0.85, 4., -1.75)));
        cuts.push((Vec3::new(-0.85, -4., -3.45), Vec3::new(0.85, -0.22, -1.75)));
    }
    if a == Aircraft::F22 {
        // Open the inlet cores through the body loft. Intake-trunk walls close
        // their perimeter; retaining the original loft here occludes the duct.
        for side in [-1.,1.]{cuts.push((Vec3::new(side*0.93-0.69,-1.30,5.45),Vec3::new(side*0.93+0.69,0.55,10.)));}
    }
    let mut pieces = if a==Aircraft::F22{points.into_iter().flat_map(raptor_detail::cut_cockpit).collect()}else{points};
    for (min, max) in cuts {
        pieces = pieces
            .into_iter()
            .flat_map(|p| crate::world::subtract_box(p, min, max))
            .collect();
    }
    if a == Aircraft::F22 { pieces = raptor_main_bay::cut(raptor_bay::cut(pieces)); }
    pieces
}
fn side_point(a: Aircraft, side: f32, y: f32, z: f32) -> Vec3 {
    let s = sections(a);
    let p = s
        .windows(2)
        .find(|p| z as f64 >= p[0].0 && z as f64 <= p[1].0)
        .unwrap();
    let t = (z as f64 - p[0].0) / (p[1].0 - p[0].0);
    let w = (p[0].1 * (1. - t) + p[1].1 * t) as f32;
    let h = (p[0].2 * (1. - t) + p[1].2 * t) as f32;
    if a==Aircraft::F22 {
        let cy=(p[0].3*(1.-t)+p[1].3*t)as f32;let target=(y-cy).abs();
        let mut lo=0_f32;let mut hi=1_f32;
        for _ in 0..24 {let x=(lo+hi)*0.5;let v=if y>=cy{h*raptor_mesh::upper(x)}else{-super::raptor::lower_profile(x,z)};if v>target{lo=x}else{hi=x}}
        return Vec3::new(side*w*(lo+hi)*0.5,y,z);
    }
    let sin = (y / h).abs().clamp(0., 1.).powf(1. / 1.6);
    Vec3::new(side * w * (1. - sin * sin).max(0.).powf(0.3), y, z)
}
fn fan_skin(x: f32, z: f32, top: bool) -> Vec3 {
    let s = sections(Aircraft::F35B);
    let p = s
        .windows(2)
        .find(|p| z as f64 >= p[0].0 && z as f64 <= p[1].0)
        .unwrap();
    let t = (z as f64 - p[0].0) / (p[1].0 - p[0].0);
    let w = (p[0].1 * (1. - t) + p[1].1 * t) as f32;
    let h = (p[0].2 * (1. - t) + p[1].2 * t) as f32;
    let y = h * (1. - (x / w).abs().powf(2. / 0.6)).max(0.).powf(0.8);
    Vec3::new(x, if top { y + 0.003 } else { -y - 0.003 }, z)
}
fn fan_door(top: bool, side: f32, hinge: Vec3) -> Mesh {
    let mut v = Vec::new();
    let mut idx = Vec::new();
    for j in 0..16 {
        for k in 0..16 {
            let point = |j: usize, k: usize| {
                let x = if top {
                    -0.85 + 1.7 * k as f32 / 16.
                } else if side < 0. {
                    -0.85 + 0.85 * k as f32 / 16.
                } else {
                    0.85 * k as f32 / 16.
                };
                fan_skin(x, -3.45 + 1.7 * j as f32 / 16., top) - hinge
            };
            let mut q = [
                point(j, k),
                point(j + 1, k),
                point(j + 1, k + 1),
                point(j, k + 1),
            ];
            if !top {
                q.reverse();
            }
            quad(&mut v, &mut idx, q);
        }
    }
    mesh(v, idx)
}
fn insignia() -> Mesh {
    let mut v = Vec::new();
    let mut idx = Vec::new();
    for i in 0..10 {
        let point = |i: usize| {
            let t = i as f32 / 10. * std::f32::consts::TAU;
            let r = if i % 2 == 0 { 0.20 } else { 0.085 };
            Vec3::new(t.sin() * r, 0., -t.cos() * r)
        };
        let n = v.len() as u32;
        v.extend([Vec3::ZERO, point(i + 1), point(i)]);
        idx.extend(n..n + 3);
    }
    mesh(v, idx)
}
fn coaming(a: Aircraft) -> Mesh {
    if a==Aircraft::F22 {return raptor_detail::sill();}
    let sections = sections(a);
    let offset = a.cabin_offset();
    let mut v = Vec::new();
    let mut idx = Vec::new();
    let point = |j: usize, side: f32, inner: bool| {
        let u = j as f32 / 48.;
        let z = offset.z - 4.08 + 2.73 * u;
        let bulge = (u * std::f32::consts::PI).sin().max(0.).sqrt();
        let width = 0.06 + 0.49 * bulge;
        if inner {
            return Vec3::new(side * width, offset.y + 0.48, z);
        }
        let pair = sections
            .windows(2)
            .find(|p| z as f64 >= p[0].0 && z as f64 <= p[1].0)
            .unwrap();
        let t = ((z as f64 - pair[0].0) / (pair[1].0 - pair[0].0)) as f32;
        let w = pair[0].1 as f32 * (1. - t) + pair[1].1 as f32 * t;
        let h = pair[0].2 as f32 * (1. - t) + pair[1].2 as f32 * t;
        let x = (width + 0.24).min(w * 0.97);
        Vec3::new(side * x, if a==Aircraft::F22{h*raptor_mesh::upper(x/w)+pair[0].3 as f32*(1.-t)+pair[1].3 as f32*t}else{h * (1. - (x / w).powi(2)).max(0.).sqrt()}, z)
    };
    for side in [-1., 1.] {
        for j in 0..48 {
            quad(
                &mut v,
                &mut idx,
                [
                    point(j, side, true),
                    point(j + 1, side, true),
                    point(j + 1, side, false),
                    point(j, side, false),
                ],
            );
        }
    }
    mesh(v, idx)
}
fn nacelle(a: Aircraft, engine: usize) -> Mesh {
    let l = a.spec().length as f32;
    let mut v = Vec::new();
    let mut idx = Vec::new();
    for j in 0..24 {
        for k in 0..48 {
            let at = |j: usize, k: usize| {
                let t = j as f32 / 24.;
                let radius = 0.48 + 0.16 * (t * std::f32::consts::PI).sin();
                let theta = k as f32 / 48. * std::f32::consts::TAU;
                Vec3::new(
                    radius * theta.cos(),
                    radius * theta.sin(),
                    -l * 0.47 + l * 0.47 * t,
                )
            };
            let origin = a.engine_positions()[engine].as_vec3();
            let q = [at(j, k), at(j, k + 1), at(j + 1, k + 1), at(j + 1, k)];
            for polygon in cut_skin(a, q.map(|p| p + origin).to_vec()) {
                polygon_into(
                    &mut v,
                    &mut idx,
                    &polygon.iter().map(|p| *p - origin).collect::<Vec<_>>(),
                );
            }
        }
    }
    mesh(v, idx)
}
fn material(
    m: &mut Assets<StandardMaterial>,
    color: Color,
    metal: f32,
) -> Handle<StandardMaterial> {
    m.add(StandardMaterial {
        base_color: color,
        metallic: metal,
        perceptual_roughness: 0.52,
        double_sided: true,
        cull_mode: None,
        ..default()
    })
}
#[derive(Component)]
pub(crate) struct ImmutableGeometry;
fn part(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    root: Entity,
    shape: Mesh,
    mat: Handle<StandardMaterial>,
    t: Transform,
) -> Entity {
    let e = c.spawn((ImmutableGeometry, Mesh3d(m.add(shape)), MeshMaterial3d(mat), t)).id();
    c.entity(root).add_child(e);
    e
}
fn box_part(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    root: Entity,
    mat: Handle<StandardMaterial>,
    p: Vec3,
    s: Vec3,
) -> Entity {
    part(
        c,
        m,
        root,
        Cuboid::from_size(s).into(),
        mat,
        Transform::from_translation(p),
    )
}
fn tube(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    root: Entity,
    mat: Handle<StandardMaterial>,
    a: Vec3,
    b: Vec3,
    r: f32,
) -> Entity {
    let d = b - a;
    part(
        c,
        m,
        root,
        Cylinder::new(r, d.length()).mesh().resolution(16).build(),
        mat,
        Transform::from_translation((a + b) * 0.5)
            .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize())),
    )
}
pub fn gear_mount(a: Aircraft, i: usize) -> DVec3 {
    let s = a.spec();
    DVec3::new(
        if i == 0 {
            0.
        } else if i == 1 {
            -1.15
        } else {
            1.15
        },
        -0.3,
        if i == 0 {
            -s.length * 0.29
        } else {
            s.length * 0.09
        },
    )
}
pub fn gear_pose(f: &Flight, i: usize, compressed: bool) -> (DVec3, DVec3, bevy::math::DQuat) {
    let at = gear_mount(f.aircraft, i);
    let length = if i == 0 { 1.40 } else { 1.45 };
    let q =
        bevy::math::DQuat::from_rotation_x((1. - f.gear_extension) * std::f64::consts::FRAC_PI_2);
    let end = at
        + q * (-DVec3::Y
            * (length
                - if compressed {
                    f.gear_compression[i]
                } else {
                    0.
                }));
    (at, end, q)
}
pub fn setup(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let f22_detail=raptor_finish::Detail::new(&mut images);
    let captive_missile=material(&mut mats,Color::WHITE,0.15);
    for a in [
        Aircraft::Su27,
        Aircraft::F35B,
        Aircraft::F22,
        Aircraft::Hornet,
    ] {
        let root = c
            .spawn((Root(a), Transform::default(), Visibility::Hidden))
            .id();
        let spec = a.spec();
        let skin = material(
            &mut mats,
            match a {
                Aircraft::Su27 => Color::srgb(0.33, 0.48, 0.60),
                Aircraft::F22 => Color::srgb(0.59, 0.61, 0.62),
                Aircraft::F35B => Color::srgb(0.27, 0.30, 0.33),
                _ => Color::srgb(0.48, 0.51, 0.53),
            },
            0.3,
        );
        let edge = material(&mut mats, Color::srgb(0.23, 0.27, 0.29), 0.5);
        let black = material(&mut mats, Color::srgb(0.018, 0.022, 0.028), 0.25);
        let silver = material(&mut mats, Color::srgb(0.58, 0.62, 0.65), 0.85);
        let ram_border = material(&mut mats,Color::srgb(0.51,0.54,0.55),0.3);
        let radome = material(
            &mut mats,
            if a == Aircraft::Su27 {
                Color::srgb(0.16, 0.27, 0.25)
            } else if a==Aircraft::F22 {Color::srgb(0.43,0.45,0.46)}else {
                Color::srgb(0.31, 0.33, 0.35)
            },
            0.15,
        );
        part(
            &mut c,
            &mut meshes,
            root,
            hull_mesh(a, false),
            skin.clone(),
            Transform::default(),
        );
        part(
            &mut c,
            &mut meshes,
            root,
            hull_mesh(a, true),
            radome,
            Transform::default(),
        );
        for side in [-1., 1.] {
            if side < 0. {
                part(
                    &mut c,
                    &mut meshes,
                    root,
                    coaming(a),
                    skin.clone(),
                    Transform::default(),
                );
            }
            let wing_pieces = if a==Aircraft::F22 {fixed_wing_with(a,side,raptor_detail::wing(side),raptor_detail::panels())}else{fixed_wing(a,side)};
            for piece in wing_pieces {
                part(
                    &mut c,
                    &mut meshes,
                    root,
                    finish(a,solid(&piece, Vec3::Y * 0.045)),
                    skin.clone(),
                    Transform::default(),
                );
            }
            if a==Aircraft::F22 {part(&mut c,&mut meshes,root,raptor_detail::border(&raptor_detail::wing(side),Vec3::Y),ram_border.clone(),Transform::default());}
            let l = spec.length as f32;
            let lex = [
                Vec3::new(side * 0.55, 0.22, -l * 0.30),
                Vec3::new(side * 1.30, 0.08, -l * 0.18),
                Vec3::new(side * 2.2, 0., l * 0.04),
                Vec3::new(side * 0.85, 0.05, l * 0.12),
            ];
            if a!=Aircraft::F22 {part(&mut c,&mut meshes,root,solid(&lex, Vec3::Y * 0.06),skin.clone(),Transform::default());}
            let (original_fin, thickness) = fin(a, side);
            let fin = if a==Aircraft::F22 {raptor_detail::fin(side)}else{original_fin.to_vec()};
            part(
                &mut c,
                &mut meshes,
                root,
                finish(a,solid(&fin, thickness)),
                skin.clone(),
                Transform::default(),
            );
            if a==Aircraft::F22 {part(&mut c,&mut meshes,root,raptor_detail::border(&fin,thickness.normalize()),ram_border.clone(),Transform::default());}
            // Intake lips and inset black ducts; Su-27 has widely separated engines.
            let x = if a == Aircraft::Su27 { 1.45 } else { 1.0 };
            let z = -l * 0.10;
            let lip = [
                Vec3::new(side * (x - 0.43), 0.18, z - 0.35),
                Vec3::new(side * (x + 0.43), 0.20, z),
                Vec3::new(side * (x + 0.39), -0.63, z + 0.15),
                Vec3::new(side * (x - 0.38), -0.60, z - 0.18),
            ];
            if a!=Aircraft::F22 {part(
                &mut c,&mut meshes,root,solid(&lip, Vec3::Z * 0.08),edge.clone(),Transform::default(),
            );}
            let centre = lip.iter().copied().sum::<Vec3>() / 4.;
            let duct = lip.map(|p| centre + (p - centre) * 0.85 - Vec3::Z * 0.095);
            if a!=Aircraft::F22 {part(&mut c,&mut meshes,root,solid(&duct, Vec3::Z * 0.006),black.clone(),Transform::default());}
            // Navigation lights remain geometry-backed and share their point position.
            let pos = Vec3::new(
                side * spec.span as f32 * 0.49,
                0.,
                if a==Aircraft::F22{3.75}else{spec.length as f32 * 0.12},
            );
            let color = if side < 0. {
                Color::srgb(1., 0.02, 0.01)
            } else {
                Color::srgb(0.01, 1., 0.15)
            };
            let glow = mats.add(StandardMaterial {
                base_color: color,
                emissive: color.to_linear() * 15.,
                ..default()
            });
            let lens = part(
                &mut c,
                &mut meshes,
                root,
                Sphere::new(0.065).mesh().uv(12, 8),
                glow,
                Transform::from_translation(pos),
            );
            c.entity(lens).insert(Lens(a));
            let e = c
                .spawn((
                    PointLight {
                        color,
                        intensity: 900.,
                        range: 7.,
                        shadows_enabled: false,
                        ..default()
                    },
                    Transform::from_translation(pos),
                    Lamp(a, 0),
                ))
                .id();
            c.entity(root).add_child(e);
        }
        for (i, p) in a
            .panels()
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, p)| p.area > 0.)
        {
            let render_panel=if a==Aircraft::F22{raptor_detail::panels()[i]}else{p};
            let (poly, thick) = if a==Aircraft::F22{raptor_detail::surface_polygon(i)}else{let(p,t)=panel_polygon(render_panel);(p.to_vec(),t)};
            let local = poly.iter().map(|v| *v - render_panel.center.as_vec3()).collect::<Vec<_>>();
            let e = part(
                &mut c,
                &mut meshes,
                root,
                if a==Aircraft::F22{raptor_mesh::paint_at(solid(&local,thick),render_panel.center.as_vec3())}else{solid(&local,thick)},
                skin.clone(),
                Transform::from_translation(render_panel.center.as_vec3()),
            );
            c.entity(e).insert(Moving(a, i));
            if a==Aircraft::F22 {part(&mut c,&mut meshes,e,raptor_detail::border(&local,p.normal.as_vec3()),ram_border.clone(),Transform::default());}
            if i == 9 || i == 10 {
                let paint = material(
                    &mut mats,
                    if a == Aircraft::Su27 {
                        Color::srgb(0.6, 0.04, 0.04)
                    } else {
                        Color::srgb(0.64, 0.66, 0.66)
                    },
                    0.05,
                );
                if a==Aircraft::F22{
                    let at=Vec3::new(p.center.x.signum() as f32*5.65,0.012,3.55);
                    for side in [-1.,1.]{box_part(&mut c,&mut meshes,root,paint.clone(),at+Vec3::X*side*0.25,Vec3::new(0.22,0.005,0.11));}
                    part(&mut c,&mut meshes,root,insignia(),paint,Transform::from_translation(at));
                }else{
                part(
                    &mut c,
                    &mut meshes,
                    e,
                    insignia(),
                    paint,
                    Transform::from_xyz(0., 0.048, p.chord as f32 * 0.45),
                );
                }
            }
            let scar = mats.add(StandardMaterial {
                base_color: Color::srgba(0.04, 0.025, 0.015, 0.8),
                alpha_mode: AlphaMode::Blend,
                cull_mode: None,
                ..default()
            });
            let e = part(
                &mut c,
                &mut meshes,
                if a==Aircraft::F22{e}else{root},
                Rectangle::new(0.5, 0.7).into(),
                scar,
                Transform::from_translation(if a==Aircraft::F22{p.normal.as_vec3()*0.06}else{p.center.as_vec3() + Vec3::Y * 0.09})
                    .with_rotation(if a==Aircraft::F22{Quat::from_rotation_arc(Vec3::Z,p.normal.as_vec3())}else{Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)}),
            );
            c.entity(e).insert(Scar(a, i));
        }
        if a==Aircraft::F22 {raptor_mesh::details(&mut c,&mut meshes,&mut mats,&f22_detail,root,skin.clone(),edge.clone(),black.clone(),silver.clone());raptor_detail::dorsal_details(&mut c,&mut meshes,&mut mats,root);}
        for (i, at) in a.engine_positions().iter().enumerate().take(spec.engines) {
            let at=if a==Aircraft::F22{raptor_detail::engine_origin(*at)}else{*at};
            if spec.engines == 2 && a!=Aircraft::F22 {
                part(
                    &mut c,
                    &mut meshes,
                    root,
                    nacelle(a, i),
                    skin.clone(),
                    Transform::from_translation(at.as_vec3()),
                );
            }
            let nozzle = c
                .spawn((
                    Nozzle(a, i),
                    Transform::from_translation(at.as_vec3()),
                    Visibility::Inherited,
                ))
                .id();
            c.entity(root).add_child(nozzle);
            // Separate turbine face, concentric rings and actuated nozzle petals.
            if a==Aircraft::F22{raptor_detail::nozzle(&mut c,&mut meshes,&mut mats,&mut images,root,nozzle,i,at.as_vec3(),skin.clone());}else{part(
                &mut c,
                &mut meshes,
                nozzle,
                Cylinder::new(0.52, 0.06).mesh().resolution(48).build(),
                black.clone(),
                Transform::from_xyz(0., 0., -0.30)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            );}
            for j in 0..if a == Aircraft::F22 { 0 } else { 24 } {
                let theta = j as f32 / 24. * std::f32::consts::TAU;
                let size = if a == Aircraft::F22 {
                    if j < 2 {
                        Vec3::new(1.35, 0.055, 1.70)
                    } else {
                        Vec3::new(0.05, 0.78, 1.70)
                    }
                } else {
                    Vec3::new(0.15, 0.045, 0.95)
                };
                let e = part(
                    &mut c,
                    &mut meshes,
                    nozzle,
                    if a==Aircraft::F22{raptor_mesh::nozzle_plate(j)}else{Cuboid::from_size(size).into()},
                    if a==Aircraft::F22{edge.clone()}else if j % 2 == 0 {
                        silver.clone()
                    } else {
                        edge.clone()
                    },
                    Transform::from_xyz(theta.cos() * 0.56, theta.sin() * 0.56, 0.15),
                );
                c.entity(e).insert(Petal(a, i, j));
            }
            for layer in 0..7 {
                let glow = mats.add(StandardMaterial {
                    base_color: if layer < 2 {
                        Color::srgba(0.65, 0.72, 1., 0.035)
                    } else {
                        Color::srgba(0.18, 0.35, 1., 0.012)
                    },
                    emissive: LinearRgba::new(3., 4., 15., 1.),
                    alpha_mode: AlphaMode::Add,
                    unlit: true,
                    cull_mode: None,
                    ..default()
                });
                let e = part(
                    &mut c,
                    &mut meshes,
                    nozzle,
                    if a==Aircraft::F22{raptor_mesh::exhaust()}else{Cone::new(0.45, 1.).mesh().resolution(32).build()},
                    glow.clone(),
                    Transform::default(),
                );
                c.entity(e)
                    .insert((Plume(a, i, layer), bevy::light::NotShadowCaster));
                if a==Aircraft::F22{c.entity(e).insert(raptor_detail::GlowLayer(i,layer,glow));}
            }
        }
        for i in 0..(spec.ammo[0] + spec.ammo[1]) as usize {
            let pos = super::station(a, i).as_vec3();
            let internal = a == Aircraft::F35B && i >= 2;
            let transform = Transform::from_translation(pos)
                .with_scale(Vec3::new(
                    if internal { 0.7 } else { 1. },
                    if internal { 0.7 } else { 1. },
                    1.,
                ))
                .with_rotation(Quat::from_rotation_z(if internal {
                    std::f32::consts::FRAC_PI_4
                } else {
                    0.
                }));
            let e = part(
                &mut c,
                &mut meshes,
                root,
                if a == Aircraft::F22 && i < 2 { crate::combat::aim9x_mesh() }
                else if i>=spec.ammo[0] as usize {crate::combat::radar_visual::body(false,true)}
                else { crate::combat::missile_mesh() },
                if a==Aircraft::F22 { captive_missile.clone() } else { silver.clone() },
                transform,
            );
            c.entity(e).insert(Store(a, i));
            if !matches!(a, Aircraft::F22 | Aircraft::F35B) || (a == Aircraft::F35B && i < 2) {
                box_part(
                    &mut c,
                    &mut meshes,
                    root,
                    skin.clone(),
                    pos + Vec3::Y * 0.3,
                    Vec3::new(0.10, 0.42, 1.15),
                );
            }
        }
        if a==Aircraft::F22 {
            for (i,side) in [-1.,1.].into_iter().enumerate(){let e=box_part(&mut c,&mut meshes,root,skin.clone(),Vec3::new(side*3.35,-0.34,0.85),Vec3::new(0.12,0.52,1.35));c.entity(e).insert(Store(a,16+i));}
        }
        if a == Aircraft::F22 {
            raptor_main_bay::setup(&mut c,&mut meshes,&mut mats,&f22_detail,root,skin.clone());
        }
        if a == Aircraft::F35B {
            box_part(
                &mut c,
                &mut meshes,
                root,
                black.clone(),
                Vec3::new(0., -0.28, 0.),
                Vec3::new(1.85, 0.02, 3.5),
            );
            for side in [-1., 1.] {
                let hinge = Vec3::new(side * 0.96, -0.54, 0.);
                let e = box_part(
                    &mut c,
                    &mut meshes,
                    root,
                    skin.clone(),
                    hinge,
                    Vec3::new(0.95, 0.035, 3.5),
                );
                c.entity(e).insert(WeaponDoor(a, side, hinge));
            }
        }
        if a == Aircraft::F22 {
            raptor_bay::setup(&mut c,&mut meshes,&mut mats,&f22_detail,root,skin.clone());
        }
        if a==Aircraft::F22 {raptor_detail::gear(&mut c,&mut meshes,&mut mats,root,skin.clone());}
        for i in 0..if a==Aircraft::F22{0}else{3} {
            let mount = gear_mount(a, i).as_vec3();
            // Open wells: roof and walls are inset into a cutout in the belly skin.
            box_part(
                &mut c,
                &mut meshes,
                root,
                black.clone(),
                mount + Vec3::new(0., if i == 0 { 0.20 } else { 0.55 }, -0.6),
                Vec3::new(0.84, 0.04, 2.5),
            );
            for side in [-1., 1.] {
                box_part(
                    &mut c,
                    &mut meshes,
                    root,
                    edge.clone(),
                    mount + Vec3::new(side * 0.42, if i == 0 { -0.06 } else { 0. }, -0.6),
                    Vec3::new(0.025, if i == 0 { 0.48 } else { 0.85 }, 2.5),
                );
                let hinge = mount + Vec3::new(side * 0.42, -0.30, -0.6);
                let e = box_part(
                    &mut c,
                    &mut meshes,
                    root,
                    skin.clone(),
                    hinge,
                    Vec3::new(0.33, 0.025, 2.5),
                );
                c.entity(e).insert(Door(a, i, side, hinge));
            }
            let leg = c
                .spawn((Leg(a, i), Transform::default(), Visibility::Inherited))
                .id();
            c.entity(root).add_child(leg);
            tube(
                &mut c,
                &mut meshes,
                leg,
                silver.clone(),
                Vec3::ZERO,
                Vec3::new(0., -1., 0.),
                0.075,
            );
            tube(
                &mut c,
                &mut meshes,
                leg,
                edge.clone(),
                Vec3::new(0.06, -0.12, 0.14),
                Vec3::new(0., -0.72, 0.),
                0.035,
            );
            tube(
                &mut c,
                &mut meshes,
                leg,
                silver.clone(),
                Vec3::new(0., -0.42, 0.),
                Vec3::new(0.13, -0.82, 0.10),
                0.025,
            );
            let tire = part(
                &mut c,
                &mut meshes,
                root,
                Torus::new(0.17, 0.34)
                    .mesh()
                    .major_resolution(32)
                    .minor_resolution(12)
                    .build(),
                black.clone(),
                Transform::default(),
            );
            c.entity(tire).insert(Tire(a, i));
        }
        // Shared functioning avionics, with an aircraft-specific enclosing cockpit
        // and controls offset in the same frame used by camera and ray picking.
        let cabin = c
            .spawn((
                CockpitRoot(a),
                Transform::from_translation(a.cabin_offset()),
                Visibility::Inherited,
            ))
            .id();
        c.entity(root).add_child(cabin);
        crate::cockpit::setup_for(
            &mut c,
            &mut meshes,
            &mut mats,
            &mut images,
            cabin,
            Vehicle::Gripen,
            a,
        );
        crate::cockpit::fleet_display(&mut c, &mut meshes, &mut mats, &mut images, cabin, a);
        if a == Aircraft::F35B {
            let gun = super::gun_origin(a).as_vec3();
            part(
                &mut c,
                &mut meshes,
                root,
                Capsule3d::new(0.18, 1.5).mesh().build(),
                edge.clone(),
                Transform::from_translation(gun + Vec3::Z * 0.7)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            );
            tube(
                &mut c,
                &mut meshes,
                root,
                black.clone(),
                gun - Vec3::Z * 0.15,
                gun + Vec3::Z * 0.35,
                0.045,
            );
        }
        if a == Aircraft::Su27 {
            let teal = material(&mut mats, Color::srgb(0.08, 0.24, 0.24), 0.15);
            box_part(
                &mut c,
                &mut meshes,
                cabin,
                teal,
                Vec3::new(0., 0.27, -3.50),
                Vec3::new(0.78, 0.15, 0.045),
            );
            for k in 0..5 {
                let (x, y) = [
                    (-0.09, 0.43),
                    (0.09, 0.43),
                    (0., 0.29),
                    (-0.09, 0.16),
                    (0.09, 0.16),
                ][k as usize];
                let at = Vec3::new(x, y, -3.46);
                part(
                    &mut c,
                    &mut meshes,
                    cabin,
                    Cylinder::new(0.061, 0.014).mesh().resolution(32).build(),
                    black.clone(),
                    Transform::from_translation(at)
                        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                );
                for tick in 0..12 {
                    let t = tick as f32 / 12. * std::f32::consts::TAU;
                    box_part(
                        &mut c,
                        &mut meshes,
                        cabin,
                        silver.clone(),
                        at + Vec3::new(t.sin() * 0.049, t.cos() * 0.049, 0.01),
                        Vec3::new(0.003, 0.010, 0.002),
                    );
                }
                let needle = c
                    .spawn((
                        Gauge(a, k),
                        Transform::from_translation(at + Vec3::Z * 0.015),
                        Visibility::Inherited,
                        crate::cockpit::InternalDisplay,
                    ))
                    .id();
                c.entity(cabin).add_child(needle);
                box_part(
                    &mut c,
                    &mut meshes,
                    needle,
                    silver.clone(),
                    Vec3::new(0., 0.023, 0.),
                    Vec3::new(0.004, 0.048, 0.003),
                );
            }
        }
        // Closed, smoothly sampled glazing fitted to the shoulder/cockpit opening.
        let glass = mats.add(StandardMaterial {
            base_color: Color::srgba(
                if a == Aircraft::F22 { 0.86 } else { 0.18 },
                if a==Aircraft::F22{0.65}else{0.32},
                if a==Aircraft::F22{0.22}else{0.32},
                if a==Aircraft::F22{0.65}else{0.18},
            ),
            alpha_mode: AlphaMode::Blend,
            metallic: if a==Aircraft::F22{0.65}else{0.12},
            perceptual_roughness: 0.12,
            cull_mode: if a==Aircraft::F22{Some(bevy::render::render_resource::Face::Back)}else{None},
            ..default()
        });
        let mut v = Vec::new();
        let mut idx = Vec::new();
        for j in 0..40 {
            for k in 0..40 {
                let point = |j: usize, k: usize| {
                    let z = -4.08 + 2.73 * j as f32 / 40.;
                    let u = j as f32 / 40.;
                    let theta = k as f32 / 40. * std::f32::consts::PI;
                    let bulge = (u * std::f32::consts::PI).sin().max(0.).sqrt();
                    if a==Aircraft::F22{return raptor_detail::canopy_point(u,theta);}
                    a.cabin_offset()
                        + Vec3::new(
                            (0.06 + 0.49 * bulge) * theta.cos(),
                            0.48 + (0.10 + 0.85 * bulge) * theta.sin(),
                            z,
                        )
                };
                quad(
                    &mut v,
                    &mut idx,
                    [
                        point(j, k),
                        point(j + 1, k),
                        point(j + 1, k + 1),
                        point(j, k + 1),
                    ],
                );
            }
        }
        if a==Aircraft::F22 {for tri in idx.chunks_exact_mut(3){tri.swap(1,2);}}
        part(
            &mut c,
            &mut meshes,
            root,
            if a==Aircraft::F22{raptor_mesh::smooth(mesh(v,idx))}else{mesh(v, idx)},
            glass,
            Transform::default(),
        );
        if a==Aircraft::F22 {raptor_detail::canopy_frame(&mut c,&mut meshes,&mut mats,root,skin.clone());}
        if a == Aircraft::F35B {
            let at = Vec3::new(0., 0.40, -2.6);
            part(
                &mut c,
                &mut meshes,
                root,
                Cylinder::new(0.82, 0.10).mesh().resolution(48).build(),
                black.clone(),
                Transform::from_translation(at),
            );
            let rotor = c
                .spawn((
                    FanSpin(a),
                    Transform::from_translation(at + Vec3::Y * 0.06),
                    Visibility::Inherited,
                ))
                .id();
            c.entity(root).add_child(rotor);
            for j in 0..18 {
                let theta = j as f32 / 18. * std::f32::consts::TAU;
                let at2 = Vec3::new(theta.cos() * 0.48, 0.02, theta.sin() * 0.48);
                let e = box_part(
                    &mut c,
                    &mut meshes,
                    rotor,
                    silver.clone(),
                    at2,
                    Vec3::new(0.50, 0.025, 0.14),
                );
                c.entity(e).insert((
                    Name::new("Lift fan blade"),
                    Transform::from_translation(at2).with_rotation(Quat::from_rotation_y(-theta)),
                ));
            }
            let hinge = fan_skin(0., -3.45, true);
            let e = part(
                &mut c,
                &mut meshes,
                root,
                fan_door(true, 0., hinge),
                skin.clone(),
                Transform::from_translation(hinge),
            );
            c.entity(e).insert(LiftDoor(a, 0., hinge));
            for side in [-1., 1.] {
                let hinge = fan_skin(side * 0.85, -2.6, false);
                let e = part(
                    &mut c,
                    &mut meshes,
                    root,
                    fan_door(false, side, hinge),
                    skin.clone(),
                    Transform::from_translation(hinge),
                );
                c.entity(e).insert(LiftDoor(a, side, hinge));
            }
        }
    }
}

pub fn update(
    f: Res<Flight>,
    time: Res<Time>,
    mut parts: Query<
        (
            Option<&Root>,
            Option<&Moving>,
            Option<&Leg>,
            Option<&Tire>,
            Option<&Door>,
            Option<&Nozzle>,
            Option<&Petal>,
            Option<&Plume>,
            Option<&LiftDoor>,
            Option<&Scar>,
            Option<&Store>,
            Option<&WeaponDoor>,
            &mut Transform,
            &mut Visibility,
        ),
        Or<(
            With<Root>,
            With<Moving>,
            With<Leg>,
            With<Tire>,
            With<Door>,
            With<Nozzle>,
            With<Petal>,
            With<Plume>,
            With<LiftDoor>,
            With<Scar>,
            With<Store>,
            With<WeaponDoor>,
        )>,
    >,
    mut lamps: Query<(&Lamp, &mut PointLight)>,
) {
    for (
        root,
        moving,
        leg,
        tire,
        door,
        nozzle,
        petal,
        plume,
        lift,
        scar,
        store,
        bay_door,
        mut t,
        mut v,
    ) in &mut parts
    {
        if let Some(Root(a)) = root {
            let active = f.vehicle == Vehicle::Gripen && f.aircraft == *a && !f.crashed;
            v.set_if_neq(if active {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
            if active {
                let mut next = *t;
                next.translation = crate::world::local(f.position, &f);
                next.rotation = crate::world::local_rot(f.orientation, &f);
                t.set_if_neq(next);
            }
            continue;
        }
        if let Some(Moving(a, i)) = moving {
            if *a != f.aircraft {
                continue;
            }
            let p = a.panels()[*i];
            let angle=crate::control_surfaces::effective_angle(&f.damage,*i,super::panel_angle(*a,*i,p,f.surface,f.flaps,f.airbrake,f.alpha));
            t.rotation = if *a==Aircraft::F22{raptor_detail::surface_rotation(*i,angle)}else{super::surface_rotation(*a,*i,angle).as_quat()};
            *v = if f.damage.surface_state[*i] == crate::combat::damage::SurfaceState::Detached {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            continue;
        }
        if let Some(Leg(a, i)) = leg {
            if *a != f.aircraft {
                continue;
            }
            let (at, end, q) = gear_pose(&f, *i, true);
            t.translation = at.as_vec3();
            t.rotation = q.as_quat();
            t.scale = Vec3::new(1., at.distance(end) as f32, 1.);
            *v = if f.gear_extension > 0.01 && f.gear_health[*i] > 0. {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            continue;
        }
        if let Some(Tire(a, i)) = tire {
            if *a != f.aircraft {
                continue;
            }
            let (_, end, _) = gear_pose(&f, *i, true);
            t.translation = end.as_vec3();
            let steer = if *i == 0 {
                -f.controls.y as f32 * 0.5 / (1. + f.airspeed as f32 * 0.1)
            } else {
                0.
            };
            t.rotation = Quat::from_rotation_y(steer * f.gear_extension as f32)
                * Quat::from_rotation_z(f.gear_extension as f32 * std::f32::consts::FRAC_PI_2);
            *v = if f.gear_extension > 0.01 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            continue;
        }
        if let Some(Door(a, i, side, hinge)) = door {
            if *a != f.aircraft {
                continue;
            }
            let opening = if f.gear_health[*i] < 0.1 {
                f.gear_door.max(0.6)
            } else {
                f.gear_door
            };
            t.rotation = Quat::from_rotation_z(*side * opening as f32 * 1.45);
            t.translation = *hinge + t.rotation * Vec3::new(-*side * 0.165, 0., 0.);
            continue;
        }
        if let Some(Nozzle(a, i)) = nozzle {
            if *a == f.aircraft {
                t.rotation = if *a==Aircraft::F22{Quat::IDENTITY}else{Quat::from_rotation_x(f.systems.nozzle[*i] as f32)};
            }
            continue;
        }
        if let Some(Petal(a, i, j)) = petal {
            if *a != f.aircraft {
                continue;
            }
            let theta = *j as f32 / 24. * std::f32::consts::TAU;
            let radius =
                (0.48 + f.spool as f32 * 0.13) * (0.98 + 0.02 * f.damage.nozzles[*i] as f32);
            if *a == Aircraft::F22 {
                t.translation = match *j {
                    0 => Vec3::new(0., radius * 0.65, 0.85),
                    1 => Vec3::new(0., -radius * 0.65, 0.85),
                    2 => Vec3::new(-0.67, 0., 0.85),
                    _ => Vec3::new(0.67, 0., 0.85),
                };
                t.rotation = Quat::from_rotation_x(if *j < 2 {
                    (if *j == 0 { -0.12 } else { 0.12 }) * (1. - f.spool as f32)
                } else {
                    0.
                });
            } else {
                t.translation = Vec3::new(theta.cos() * radius, theta.sin() * radius, 0.15);
                t.rotation = Quat::from_rotation_z(theta - std::f32::consts::FRAC_PI_2);
            }
            continue;
        }
        if let Some(Plume(a, i, layer)) = plume {
            if *a != f.aircraft {
                continue;
            }
            let active =
                f.engine && f.thrust > 1. && f.systems.engines[*i] > 0.1 && f.systems.engine_on[*i];
            *v = if active {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            let wet = ((f.spool - 0.88) / 0.12).clamp(0., 1.) as f32;
            let length = 0.6 + f.spool as f32 * 0.8 + wet * 4.;
            let l = *layer as f32;
            let motion = crate::plume_motion::sample(f.elapsed, (*i * 37 + *layer) as u32);
            if *a == Aircraft::F22 {
                // A narrow hot core sits inside a longer blue envelope. All
                // layers originate in the recessed duct, before the exit lip.
                let length = (0.6 + f.spool as f32 * 0.8 + wet * 7.0)
                    * if *layer < 2 { 0.65 + l * 0.15 } else { 1. - (l - 2.) * 0.035 }
                    * motion.length;
                let radius = (if *layer < 2 { 0.34 + l * 0.16 } else { 0.80 + (l - 2.) * 0.08 }) * motion.width;
                let vector=Quat::from_rotation_x(f.systems.nozzle[*i] as f32) * motion.bend;
                // Fixed interior origin; bend the downstream jet with TVC.
                t.translation = Vec3::Z*raptor_detail::EXHAUST_START + vector*(Vec3::Z*length*0.5);
                t.rotation = vector*Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
                t.scale = Vec3::new(radius, length, radius * 0.65);
                continue;
            }
            let length = length * (1. - l * 0.06) * motion.length;
            t.translation = Vec3::Z * (0.65 + l * 0.08) + motion.bend * (Vec3::Z * length * 0.5);
            t.rotation = motion.bend * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
            t.scale = Vec3::new(
                (1. - l * 0.08) * (1. + wet * 0.25) * motion.width,
                length,
                motion.width,
            );
            continue;
        }
        if let Some(LiftDoor(a, side, hinge)) = lift {
            if *a != f.aircraft {
                continue;
            }
            t.rotation = if *side == 0. {
                Quat::from_rotation_x(-f.systems.conversion as f32 * 1.65)
            } else {
                Quat::from_rotation_z(*side * f.systems.conversion as f32 * 1.45)
            };
            t.translation = *hinge;
            continue;
        }
        if let Some(Store(a, i)) = store {
            if *a==Aircraft::F22 {
                if *i>=16 {*v=if *a==f.aircraft&&f.systems.external_stores{Visibility::Inherited}else{Visibility::Hidden};continue;}
                if *i<2 {
                    t.set_if_neq(super::bays::missile_pose(*i,f.systems.side_bays[*i].launcher));
                    v.set_if_neq(if *a==f.aircraft&&f.systems.stores[*i]{Visibility::Inherited}else{Visibility::Hidden});continue;
                }
                let external=f.systems.external_stores&&*i>=6;
                t.translation=if external{super::station_for(&f,*i).as_vec3()}else{super::main_bay::store_position(*i-2,f.systems.main_stroke[*i-2]).as_vec3()};
                t.scale=Vec3::ONE;
                *v=if *a==f.aircraft&&f.systems.stores[*i]{Visibility::Inherited}else{Visibility::Hidden};continue;
            }
            *v = if *a == f.aircraft && f.systems.stores[*i] {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            continue;
        }
        if let Some(WeaponDoor(a, side, hinge)) = bay_door {
            if *a != f.aircraft {
                continue;
            }
            t.rotation = Quat::from_rotation_z(*side * f.systems.bays as f32 * 1.5);
            t.translation = if *a == Aircraft::F22 { *hinge } else { *hinge + t.rotation * Vec3::new(-*side * 0.475, 0., 0.) };
            continue;
        }
        if let Some(Scar(a, i)) = scar {
            let damage = 1. - f.damage.surfaces[*i];
            *v = if *a == f.aircraft && damage > 0.05 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            t.scale = Vec3::splat((damage * 1.5) as f32);
        }
    }
    for (Lamp(a, kind), mut lamp) in &mut lamps {
        let intensity = if *a == f.aircraft && f.lights && f.battery {
            if *kind == 0 {
                900.
            } else {
                (time.elapsed_secs() * 2.).sin().max(0.) * 2000.
            }
        } else {
            0.
        };
        if lamp.intensity != intensity {
            lamp.intensity = intensity;
        }
    }
}
pub fn gauges(
    f: Res<Flight>,
    mut q: Query<(&Gauge, &mut Transform), Without<FanSpin>>,
    mut fans: Query<(&FanSpin, &mut Transform), Without<Gauge>>,
    mut lenses: Query<(&Lens, &mut Visibility)>,
) {
    for (Lens(a), mut v) in &mut lenses {
        v.set_if_neq(if *a == f.aircraft && f.lights && f.battery {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    for (FanSpin(a), mut t) in &mut fans {
        if *a == f.aircraft && f.engine && f.systems.engine_on[0] && f.damage.lift_fan > 0.1 {
            t.rotation =
                Quat::from_rotation_y((f.elapsed * f.spool * f.systems.conversion * 70.) as f32);
        }
    }
    for (Gauge(a, k), mut t) in &mut q {
        if *a != f.aircraft {
            continue;
        }
        let value = match k {
            0 => f.airspeed / 350.,
            1 => f.altitude() / 12000.,
            2 => f.spool,
            3 => f.fuel / f.aircraft.spec().fuel,
            _ => f.alpha.to_degrees() / 90.,
        };
        t.rotation =
            Quat::from_rotation_z((-2.5 + value.clamp(0., 1.) as f32 * 5.).clamp(-2.5, 2.5));
    }
}

#[cfg(test)]
mod plume_tests {
    use super::*;
    #[test]
    fn raptor_plumes_move_independently_but_keep_their_recessed_origins() {
        let mut app = App::new();
        let mut f = Flight::runway();
        super::super::select(&mut f, Aircraft::F22);
        f.spool = 1.; f.throttle = 1.; f.thrust = 312000.; f.engine = true;
        f.systems.nozzle = [0.2, -0.1];
        app.add_plugins(MinimalPlugins).insert_resource(f).add_systems(Update, update);
        let a = app.world_mut().spawn((Plume(Aircraft::F22, 0, 2), Transform::default(), Visibility::Inherited)).id();
        let b = app.world_mut().spawn((Plume(Aircraft::F22, 1, 2), Transform::default(), Visibility::Inherited)).id();
        let mut poses = Vec::new();
        for elapsed in [0., 0.037, 0.037] {
            app.world_mut().resource_mut::<Flight>().elapsed = elapsed;
            app.update();
            let t = *app.world().get::<Transform>(a).unwrap();
            let root = t.transform_point(Vec3::new(0., -0.5, 0.));
            assert!(root.distance(Vec3::Z * raptor_detail::EXHAUST_START) < 1e-5);
            assert_ne!(t.scale, app.world().get::<Transform>(b).unwrap().scale);
            poses.push(t);
        }
        assert_ne!(poses[0], poses[1]);
        assert_eq!(poses[1], poses[2]);
    }
}
