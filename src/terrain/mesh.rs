//! Ground mesh construction; source coordinates stay in f64 until anchoring.
use super::*;
fn geographic_uv(position: DVec3) -> [f32; 2] {
    let (lon, lat) = crate::environment::local_to_geo(position);
    // Subtract in f64 before upload: absolute latitude loses ~0.4 m of
    // precision in f32 and makes near-ground texture derivatives collapse.
    [(lon - 12.711111) as f32, (lat - 58.428333) as f32]
}
pub(crate) fn terrain_mesh(x: f64, z: f64, size: f64, n: usize, anchor: DVec3) -> Mesh {
    let mut p = Vec::new();
    let mut colors = Vec::new();
    let mut uv = Vec::new();
    let mut surface_uv = Vec::new();
    let mut idx = Vec::new();
    let mut surface_normals = Vec::new();
    for j in 0..=n {
        for i in 0..=n {
            let xx = x + size * i as f64 / n as f64;
            let zz = z + size * j as f64 / n as f64;
            let h = terrain_height(xx, zz);
            p.push(
                (crate::terrain::point(xx, zz, h) - anchor)
                    .as_vec3()
                    .to_array(),
            );
            let position = crate::terrain::point(xx, zz, h);
            if crate::environment::real() {
                // Geographic interpolation happens per vertex, not through
                // millions of atan2 evaluations in the fragment shader.
                uv.push(geographic_uv(position));
            } else {
                uv.push([xx as f32 * 0.025, zz as f32 * 0.025]);
            }
            let sample = crate::environment::active().sample_surface(position);
            let slope = 1. - sample.normal.dot(position.normalize());
            let normal = if crate::environment::real() {
                sample.normal
            } else {
                crate::terrain::normal(crate::terrain::point(xx, zz, 0.))
            };
            surface_normals.push(normal.as_vec3().to_array());
            colors.push(if crate::environment::real() {
                crate::environment::surface_color_sampled(xx, zz, h, position.normalize(), sample)
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
    for &index in &perimeter {
        let mut vertex = p[index];
        vertex[1] -= 25.0;
        p.push(vertex);
        uv.push(uv[index]);
        surface_uv.push(surface_uv[index]);
        colors.push(colors[index]);
    }
    for i in 0..perimeter.len() {
        let next = (i + 1) % perimeter.len();
        let a = perimeter[i] as u32;
        let b = perimeter[next] as u32;
        let c = (skirt_start + i) as u32;
        let d = (skirt_start + next) as u32;
        idx.extend([a, c, b, b, c, d]);
    }
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_1, surface_uv);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    m.insert_indices(Indices::U32(idx));
    m.compute_smooth_normals();
    // Keep skirt faces out of visible edge normals, including at LOD changes.
    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
        m.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
    {
        normals[..surface_normals.len()].copy_from_slice(&surface_normals);
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
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
