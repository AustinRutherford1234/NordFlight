//! Ground mesh construction; source coordinates stay in f64 until anchoring.
use super::*;
fn geographic_uv(position: DVec3) -> [f32; 2] {
    let (lon, lat) = crate::environment::local_to_geo(position);
    // Subtract in f64 before upload: absolute latitude loses ~0.4 m of
    // precision in f32 and makes near-ground texture derivatives collapse.
    [(lon - 12.711111) as f32, (lat - 58.428333) as f32]
}
pub(crate) fn terrain_mesh(x: f64, z: f64, size: f64, n: usize, anchor: DVec3) -> Mesh {
    let vertex_count = (n + 1) * (n + 1) + 4 * n;
    let mut p = Vec::with_capacity(vertex_count);
    let mut colors = Vec::with_capacity(vertex_count);
    let mut uv = Vec::with_capacity(vertex_count);
    let mut surface_uv = Vec::with_capacity(vertex_count);
    let mut idx = Vec::with_capacity(6 * n * (n + 4));
    let mut surface_normals = Vec::with_capacity(vertex_count);
    let world = crate::environment::active();
    let real = !world.preset.legacy;
    for j in 0..=n {
        for i in 0..=n {
            let xx = x + size * i as f64 / n as f64;
            let zz = z + size * j as f64 / n as f64;
            let h = terrain_height(xx, zz);
            let position = crate::terrain::point(xx, zz, h);
            p.push((position - anchor).as_vec3().to_array());
            if real {
                // Geographic interpolation happens per vertex, not through
                // millions of atan2 evaluations in the fragment shader.
                uv.push(geographic_uv(position));
            } else {
                uv.push([xx as f32 * 0.025, zz as f32 * 0.025]);
            }
            let sample = world.sample_surface(position);
            let up = position.normalize();
            let slope = 1. - sample.normal.dot(up);
            let normal = if real {
                sample.normal
            } else {
                crate::terrain::normal(crate::terrain::point(xx, zz, 0.))
            };
            surface_normals.push(normal.as_vec3().to_array());
            colors.push(if real {
                crate::environment::surface_color_sampled(xx, zz, h, up, sample)
            } else {
                crate::terrain::color(xx, zz, h)
            });
            // Slope is a continuous geographic attribute. A face derivative
            // would expose mesh triangles when blending rock at coarse LOD.
            surface_uv.push([slope as f32, h as f32]);
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = (j * (n + 1) + i) as u32;
            let b = a + 1;
            let d = a + (n + 1) as u32;
            let cc = d + 1;
            idx.extend([a, d, b, b, d, cc]);
        }
    }
    // Skirts conceal T-junctions where neighboring tiles use different grids.
    let perimeter: Vec<usize> = (0..=n)
        .chain((1..=n).map(|j| j * (n + 1) + n))
        .chain((0..n).rev().map(|i| n * (n + 1) + i))
        .chain((1..n).rev().map(|j| j * (n + 1)))
        .collect();
    let skirt_start = p.len();
    let mut skirt_positions: Vec<_> = perimeter.iter().map(|&index| p[index]).collect();
    skirt_positions.reserve(perimeter.len());
    for &index in &perimeter {
        let mut vertex = p[index];
        vertex[1] -= 25.0;
        p.push(vertex);
        skirt_positions.push(vertex);
        uv.push(uv[index]);
        surface_uv.push(surface_uv[index]);
        colors.push(colors[index]);
    }
    let edge = perimeter.len();
    let mut skirt_indices = Vec::with_capacity(edge * 6);
    for i in 0..edge {
        let next = (i + 1) % edge;
        skirt_indices.extend([i, edge + i, next, next, edge + i, edge + next].map(|v| v as u32));
        let a = perimeter[i];
        let b = perimeter[next];
        let c = skirt_start + i;
        let d = skirt_start + next;
        idx.extend([a, c, b, b, c, d].map(|v| v as u32));
    }
    // Only the skirt needs triangle-derived normals. The ground already has
    // source normals, including the vertices along the tile edge.
    let mut skirt = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, skirt_positions)
        .with_inserted_indices(Indices::U32(skirt_indices));
    skirt.compute_smooth_normals();
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
        skirt.attribute(Mesh::ATTRIBUTE_NORMAL)
    else {
        unreachable!("smooth normals are Float32x3");
    };
    surface_normals.extend_from_slice(&normals[edge..]);
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_1, surface_uv);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, surface_normals);
    m.insert_indices(if vertex_count <= usize::from(u16::MAX) + 1 {
        Indices::U16(idx.into_iter().map(|index| index as u16).collect())
    } else {
        Indices::U32(idx)
    });
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_terrain_mesh_keeps_skirt_normals_and_wide_index_fallback() {
        use bevy::mesh::VertexAttributeValues::Float32x3;
        for n in [64, 256] {
            let mut mesh = terrain_mesh(2048., 4096., 2048., n, point(3072., 5120., 0.));
            let indices = mesh.indices().unwrap();
            assert_eq!(matches!(indices, Indices::U16(_)), n == 64);
            assert_eq!(indices.len(), 6 * n * (n + 4));
            assert!(indices.iter().all(|i| i < mesh.count_vertices()));
            let Some(Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL) else {
                panic!("missing normals");
            };
            let skirt_start = (n + 1) * (n + 1);
            let skirt = normals[skirt_start..].to_vec();
            mesh.compute_smooth_normals();
            let Some(Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL) else {
                panic!("missing reference normals");
            };
            assert_eq!(skirt, normals[skirt_start..]);
        }
    }
    #[test]
    fn regional_uvs_resolve_centimetres_before_gpu_upload() {
        let p = crate::environment::geo_to_local(12.711111, 58.428333, 0., 6371000.);
        let q = crate::environment::geo_to_local(12.711111 + 1e-7, 58.428333 + 1e-7, 0., 6371000.);
        let a = geographic_uv(p);
        let b = geographic_uv(q);
        for i in 0..2 {
            assert!(((b[i] - a[i]) as f64 - 1e-7).abs() < 1e-11);
        }
    }
}
