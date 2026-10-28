//! Smooth, continuous organic surfaces, generated once and cached with the rig.
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use std::collections::HashMap;
pub fn ellipsoid(p: Vec3, c: Vec3, r: Vec3) -> f32 {
    let v = p - c;
    let a = (v / r).length();
    let b = (v / (r * r)).length();
    if b < 1e-6 {
        -r.min_element()
    } else {
        a * (a - 1.) / b
    }
}
pub fn union(a: f32, b: f32, k: f32) -> f32 {
    let h = ((k - (a - b).abs()) / k).max(0.);
    a.min(b) - h * h * k * 0.25
}
pub fn subtract(a: f32, b: f32, k: f32) -> f32 {
    -union(-a, b, k)
}
pub fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
pub fn surface(
    bounds: Rect3,
    step: f32,
    sdf: impl Fn(Vec3) -> f32,
    color: impl Fn(Vec3) -> [f32; 4],
) -> Mesh {
    let dims = ((bounds.max - bounds.min) / step).ceil().as_uvec3() + UVec3::ONE;
    let stride = dims.x as usize;
    let plane = stride * dims.y as usize;
    let at = |x: u32, y: u32, z: u32| bounds.min + Vec3::new(x as f32, y as f32, z as f32) * step;
    let id = |x: u32, y: u32, z: u32| x as usize + y as usize * stride + z as usize * plane;
    let mut values = vec![0.; plane * dims.z as usize];
    for z in 0..dims.z {
        for y in 0..dims.y {
            for x in 0..dims.x {
                values[id(x, y, z)] = sdf(at(x, y, z));
            }
        }
    }
    let normal = |p: Vec3| {
        let e = 0.0006;
        Vec3::new(
            sdf(p + Vec3::X * e) - sdf(p - Vec3::X * e),
            sdf(p + Vec3::Y * e) - sdf(p - Vec3::Y * e),
            sdf(p + Vec3::Z * e) - sdf(p - Vec3::Z * e),
        )
        .normalize_or(Vec3::Y)
    };
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut colors = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let mut edges = HashMap::<(usize, usize), u32>::new();
    let corners = [
        UVec3::new(0, 0, 0),
        UVec3::new(1, 0, 0),
        UVec3::new(1, 1, 0),
        UVec3::new(0, 1, 0),
        UVec3::new(0, 0, 1),
        UVec3::new(1, 0, 1),
        UVec3::new(1, 1, 1),
        UVec3::new(0, 1, 1),
    ];
    let tetra = [
        [0, 5, 1, 6],
        [0, 1, 2, 6],
        [0, 2, 3, 6],
        [0, 3, 7, 6],
        [0, 7, 4, 6],
        [0, 4, 5, 6],
    ];
    for z in 0..dims.z - 1 {
        for y in 0..dims.y - 1 {
            for x in 0..dims.x - 1 {
                let c = corners.map(|v| UVec3::new(x, y, z) + v);
                let ids = c.map(|v| id(v.x, v.y, v.z));
                let f = ids.map(|i| values[i]);
                if f.iter().all(|v| *v >= 0.) || f.iter().all(|v| *v < 0.) {
                    continue;
                }
                let p = c.map(|v| at(v.x, v.y, v.z));
                for t in tetra {
                    let mut polygon = Vec::with_capacity(4);
                    for [a, b] in [
                        [t[0], t[1]],
                        [t[0], t[2]],
                        [t[0], t[3]],
                        [t[1], t[2]],
                        [t[1], t[3]],
                        [t[2], t[3]],
                    ] {
                        if (f[a] < 0.) == (f[b] < 0.) {
                            continue;
                        }
                        let key = if ids[a] < ids[b] {
                            (ids[a], ids[b])
                        } else {
                            (ids[b], ids[a])
                        };
                        let vertex = *edges.entry(key).or_insert_with(|| {
                            let q = p[a].lerp(p[b], (f[a] / (f[a] - f[b])).clamp(0., 1.));
                            let n = normal(q);
                            let i = positions.len() as u32;
                            positions.push(q.to_array());
                            normals.push(n.to_array());
                            colors.push(color(q));
                            uvs.push([q.x * 2. + 0.5, q.y * 2. + 0.5]);
                            i
                        });
                        polygon.push(vertex);
                    }
                    if polygon.len() < 3 {
                        continue;
                    }
                    let centre = polygon
                        .iter()
                        .map(|i| Vec3::from(positions[*i as usize]))
                        .sum::<Vec3>()
                        / polygon.len() as f32;
                    let n = normal(centre);
                    let u = n.any_orthonormal_vector();
                    let v = n.cross(u);
                    polygon.sort_by(|a, b| {
                        let a = Vec3::from(positions[*a as usize]) - centre;
                        let b = Vec3::from(positions[*b as usize]) - centre;
                        a.dot(v)
                            .atan2(a.dot(u))
                            .total_cmp(&b.dot(v).atan2(b.dot(u)))
                    });
                    for i in 1..polygon.len() - 1 {
                        let mut tri = [polygon[0], polygon[i], polygon[i + 1]];
                        let q = tri.map(|i| Vec3::from(positions[i as usize]));
                        if (q[1] - q[0]).cross(q[2] - q[0]).dot(n) < 0. {
                            tri.swap(1, 2);
                        }
                        indices.extend(tri);
                    }
                }
            }
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
pub struct Rect3 {
    pub min: Vec3,
    pub max: Vec3,
}
