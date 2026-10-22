//! Raptor intake trunks, blended engine deck and operational surface finish.
use super::*;
pub(super) fn exhaust() -> Mesh {
    let mut p = Vec::new();
    let mut idx = Vec::new();
    let mut colors = Vec::new();
    // Open rectangular stream tube. No opaque cone cap across the nozzle exit.
    for j in 0..18 {
        for k in 0..16 {
            let at = |j: usize, k: usize| {
                let u = j as f32 / 18.;
                let t = k as f32 / 16. * std::f32::consts::TAU;
                let width = (1. - u * 0.72) * (0.91 + 0.09 * (u * 37.7).cos());
                Vec3::new(
                    t.cos().signum() * t.cos().abs().powf(0.35) * 0.65 * width,
                    u - 0.5,
                    t.sin().signum() * t.sin().abs().powf(0.35) * 0.48 * width,
                )
            };
            quad(
                &mut p,
                &mut idx,
                [at(j, k), at(j + 1, k), at(j + 1, k + 1), at(j, k + 1)],
            );
            for u in [
                j as f32 / 18.,
                (j + 1) as f32 / 18.,
                (j + 1) as f32 / 18.,
                j as f32 / 18.,
            ] {
                colors.push([1., 1., 1., (1. - u).powi(2)]);
            }
        }
    }
    let mut m = mesh(p, idx);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    m
}
pub(super) fn upper(x: f32) -> f32 {
    super::super::raptor::upper_profile(x)
}
pub(super) fn nozzle_plate(j: usize) -> Mesh {
    if j < 2 {
        let mut p = vec![
            Vec3::new(-0.675, 0., -0.85),
            Vec3::new(0.675, 0., -0.85),
            Vec3::new(0.675, 0., 0.70),
        ];
        for k in (0..=12).rev() {
            p.push(Vec3::new(
                -0.675 + 1.35 * k as f32 / 12.,
                0.,
                if k % 2 == 0 { 0.85 } else { 0.72 },
            ));
        }
        solid(&p, Vec3::Y * 0.025)
    } else {
        solid(
            &[
                Vec3::new(0., -0.39, -0.85),
                Vec3::new(0., 0.39, -0.85),
                Vec3::new(0., 0.30, 0.85),
                Vec3::new(0., -0.30, 0.85),
            ],
            Vec3::X * 0.024,
        )
    }
}
pub(super) fn smooth(mut mesh: Mesh) -> Mesh {
    use std::collections::HashMap;
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(points)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return mesh;
    };
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
        mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
    else {
        return mesh;
    };
    let key = |p: &[f32; 3]| p.map(|v| (v * 10000.).round() as i32);
    let mut sums = HashMap::<[i32; 3], Vec3>::new();
    for (p, n) in points.iter().zip(normals) {
        *sums.entry(key(p)).or_default() += Vec3::from_array(*n);
    }
    let n = points
        .iter()
        .map(|p| sums[&key(p)].normalize_or_zero().to_array())
        .collect::<Vec<_>>();
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, n);
    mesh
}
pub(super) fn paint(mesh: Mesh) -> Mesh {
    paint_at(mesh, Vec3::ZERO)
}
pub(super) fn paint_at(mut mesh: Mesh, offset: Vec3) -> Mesh {
    // Subdivide only the small planar panels so broad camouflage patches have
    // interior samples. Positions stay exactly on the original triangles.
    if mesh.count_vertices() < 500 {
        if let (Some(bevy::mesh::VertexAttributeValues::Float32x3(points)), Some(indices)) =
            (mesh.attribute(Mesh::ATTRIBUTE_POSITION), mesh.indices())
        {
            let mut v = Vec::new();
            let mut idx = Vec::new();
            let inds = indices.iter().collect::<Vec<_>>();
            for t in inds.chunks_exact(3) {
                let p = [
                    Vec3::from_array(points[t[0]]),
                    Vec3::from_array(points[t[1]]),
                    Vec3::from_array(points[t[2]]),
                ];
                let n = 7;
                let at = |i: usize, j: usize| {
                    p[0] + (p[1] - p[0]) * (i as f32 / n as f32)
                        + (p[2] - p[0]) * (j as f32 / n as f32)
                };
                for i in 0..n {
                    for j in 0..n - i {
                        polygon_into(&mut v, &mut idx, &[at(i, j), at(i + 1, j), at(i, j + 1)]);
                        if i + j + 1 < n {
                            polygon_into(
                                &mut v,
                                &mut idx,
                                &[at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)],
                            );
                        }
                    }
                }
            }
            mesh = super::mesh(v, idx);
        }
    }
    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(points)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    {
        let colors = points
            .iter()
            .map(|p| {
                let x = (p[0] + offset.x).abs();
                let z = p[2] + offset.z;
                let warp=0.10*(z*2.7+x*3.9).sin()+0.07*(z*5.3-x*2.4).sin();
                let patch = (((x - 2.2).powi(2) / 2.5 + (z + 0.9).powi(2) / 5.5)
                    .min((x - 0.44).powi(2) / 0.32 + (z - 1.0).powi(2) / 22.)
                    .min((x - 4.75).powi(2) / 0.85 + (z - 2.30).powi(2) / 0.52)
                    .min((x - 3.20).powi(2) / 0.70 + (z - 6.62).powi(2) / 0.50))+warp;
                let t = ((patch - 0.73) / 0.24).clamp(0., 1.);
                let t = t * t * (3. - 2. * t);
                let wear=0.012*(x*31.+z*23.).sin()*(x*13.-z*29.).sin();
                let mut tone = 0.58 + 0.42 * t+wear;
                let y=p[1]+offset.y;
                if y>0.85 && z>2.5 && z<6.5 {tone*=0.71+0.29*((z-4.55).abs()/1.5).clamp(0.,1.);}
                [tone*0.98, tone, tone*1.015, 1.]
            })
            .collect::<Vec<[f32; 4]>>();
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    mesh
}
pub(super) fn details(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    root: Entity,
    skin: Handle<StandardMaterial>,
    _edge: Handle<StandardMaterial>,
    black: Handle<StandardMaterial>,
    _silver: Handle<StandardMaterial>,
) {
    let ram = material(mats, Color::srgb(0.45, 0.47, 0.47), 0.18);
    raptor_intake::setup(c,m,mats,root,skin.clone(),ram.clone());
    for side in [-1., 1.] {
        // Low-visibility fin code, laid directly in the canted fin plane.
        let up = Vec3::new(side * 0.4848, 0.8746, 0.);
        let across = -Vec3::Z * side;
        let base = Vec3::new(side * 2.46, 2.0, 4.30) + Vec3::new(side * 0.025, -0.014, 0.);
        for letter in 0..2 {
            let b = base + across * (letter as f32 * 0.28);
            for (p, q) in [
                (Vec2::ZERO, Vec2::new(0., 0.36)),
                (Vec2::new(0., 0.36), Vec2::new(0.20, 0.36)),
                (Vec2::new(0., 0.19), Vec2::new(0.16, 0.19)),
            ] {
                tube(
                    c,
                    m,
                    root,
                    ram.clone(),
                    b + across * p.x + up * p.y,
                    b + across * q.x + up * q.y,
                    0.012,
                );
            }
        }
        let mut v=Vec::new();let mut ix=Vec::new();
        for j in 0..32 {for k in 0..16 {
            let point=|j:usize,k:usize|{let u=j as f32/32.;let t=k as f32/16.;let z=1.8+4.4*u;Vec3::new(side*(0.10+(1.78-0.27*u)*t),0.43-0.18*u+0.15*(t*std::f32::consts::PI).sin()*(u*std::f32::consts::PI).sin(),z)};
            quad(&mut v,&mut ix,[point(j,k),point(j+1,k),point(j+1,k+1),point(j,k+1)]);
        }}
        part(c,m,root,paint(smooth(mesh(v,ix))),skin.clone(),Transform::default());
        for z in [-3.0, -1.45, 0.10, 1.65] {
            tube(
                c,
                m,
                root,
                ram.clone(),
                Vec3::new(side * 0.73, 0.58, z),
                Vec3::new(side * 1.42, 0.44, z + 0.30),
                0.009,
            );
        }
        let gun = Vec3::new(side * 0.82, 0.43, -3.58);
        if side > 0. {
            part(
                c,
                m,
                root,
                solid(
                    &[
                        gun,
                        gun + Vec3::new(0.12, 0., 0.05),
                        gun + Vec3::new(0.12, 0., 0.65),
                        gun + Vec3::Z * 0.60,
                    ],
                    Vec3::Y * 0.006,
                ),
                black.clone(),
                Transform::default(),
            );
        }
        let glow = mats.add(StandardMaterial {
            base_color: Color::srgb(0.44, 0.53, 0.43),
            emissive: LinearRgba::new(0.25, 0.35, 0.18, 1.),
            ..default()
        });
        let e = box_part(
            c,
            m,
            root,
            glow,
            Vec3::new(side * 6.66, 0.012, 4.00),
            Vec3::new(0.025, 0.012, 0.34),
        );
        c.entity(e).insert(Lens(Aircraft::F22));
    }
}
