//! Original low-poly representative naval ordnance. Metre units, -Z nose.
//! Dimensions are representative class silhouettes, not a specific block replica.
use super::*;
use bevy::{asset::RenderAssetUsages, mesh::Indices, render::render_resource::PrimitiveTopology};
pub fn dimensions(kind: Kind) -> (f32, f32) {
    match kind {
        Kind::Standard => (4.72, 0.343),
        Kind::Essm => (3.66, 0.254),
        Kind::AntiShip => (6.2, 0.52),
        Kind::AirToAir => (3.7, 0.178),
    }
}
pub fn mesh(kind: Kind) -> Mesh {
    let (length, diameter) = dimensions(kind);
    let r = diameter * 0.5;
    let sides = 20;
    let mut p = Vec::new();
    let mut colors = Vec::new();
    let mut ix = Vec::new();
    let body = [0.72, 0.75, 0.73, 1.];
    let dark = [0.055, 0.065, 0.07, 1.];
    let seam = [0.32, 0.35, 0.35, 1.];
    let rings = [
        (-0.5, 0.0, dark),
        (-0.48, 0.18, dark),
        (-0.44, 0.55, body),
        (-0.37, 0.92, body),
        (-0.33, 1., body),
        (-0.325, 1.01, seam),
        (-0.32, 1., body),
        (-0.22, 1., body),
        (-0.215, 1.005, [0.65, 0.42, 0.13, 1.]),
        (-0.205, 1.005, [0.65, 0.42, 0.13, 1.]),
        (-0.20, 1., body),
        (0.10, 1., body),
        (0.105, 1.01, seam),
        (0.11, 1., body),
        (0.38, 1., body),
        (0.39, 1.01, seam),
        (0.40, 1., body),
        (0.47, 1., body),
        (0.49, 0.9, seam),
        (0.49, 0.65, dark),
        (0.44, 0.5, dark),
        (0.44, 0.0, dark),
    ];
    for (z, radius, color) in rings {
        for i in 0..sides {
            let a = i as f32 * std::f32::consts::TAU / sides as f32;
            p.push(Vec3::new(
                a.cos() * r * radius,
                a.sin() * r * radius,
                z * length,
            ));
            colors.push(color);
        }
    }
    for k in 0..rings.len() - 1 {
        for j in 0..sides {
            let a = (k * sides + j) as u32;
            let b = (k * sides + (j + 1) % sides) as u32;
            ix.extend([
                a,
                b,
                a + sides as u32,
                b,
                b + sides as u32,
                a + sides as u32,
            ]);
        }
    }
    let span = match kind {
        Kind::Standard => 0.55,
        Kind::Essm => 0.36,
        Kind::AirToAir => 0.24,
        Kind::AntiShip => 0.42,
    };
    for i in 0..4 {
        let before = p.len();
        let t = Transform::from_rotation(Quat::from_rotation_z(
            i as f32 * std::f32::consts::FRAC_PI_2,
        ));
        super::super::aim9x::fin(
            &mut p,
            &mut ix,
            &[
                (r * 0.9, length * 0.22),
                (span, length * 0.36),
                (span, length * 0.46),
                (r * 0.9, length * 0.46),
            ],
            0.008,
            t,
        );
        if kind != Kind::AntiShip {
            super::super::aim9x::fin(
                &mut p,
                &mut ix,
                &[
                    (r * 0.9, -length * 0.08),
                    (span * 0.85, length * 0.20),
                    (r * 0.9, length * 0.29),
                ],
                0.006,
                t,
            );
        }
        colors.resize(p.len(), [0.48, 0.52, 0.52, 1.]);
        assert!(p.len() > before);
    }
    if kind == Kind::AntiShip {
        for side in [-1., 1.] {
            let q = Transform::from_rotation(if side > 0. {
                Quat::IDENTITY
            } else {
                Quat::from_rotation_z(std::f32::consts::PI)
            });
            super::super::aim9x::fin(
                &mut p,
                &mut ix,
                &[
                    (r * 0.8, -length * 0.10),
                    (1.25, length * 0.07),
                    (1.25, length * 0.17),
                    (r * 0.8, length * 0.13),
                ],
                0.018,
                q,
            );
            colors.resize(p.len(), body);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        p.iter().map(|p| p.to_array()).collect::<Vec<_>>(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(ix));
    mesh.compute_normals();
    mesh
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_weapon_has_finite_colored_nose_fins_and_recessed_nozzle_at_class_scale() {
        for kind in [Kind::Standard, Kind::Essm, Kind::AntiShip, Kind::AirToAir] {
            let m = mesh(kind);
            let bevy::mesh::VertexAttributeValues::Float32x3(p) =
                m.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
            else {
                panic!()
            };
            let (l, d) = dimensions(kind);
            assert!(p.iter().all(|v| v.iter().all(|x| x.is_finite())));
            assert!(p.iter().any(|v| v[0].hypot(v[1]) > d));
            assert!(p.iter().any(|v| v[2] < -0.49 * l));
            assert!(p.iter().any(|v| v[2] > 0.48 * l));
            assert!(m.count_vertices() < 1200);
            assert!(m.attribute(Mesh::ATTRIBUTE_COLOR).is_some());
        }
    }
}
/// A shared 192-triangle, vertex-faded exhaust; no aircraft-resolution plume mesh.
pub fn exhaust() -> Mesh {
    let mut p = vec![];
    let mut colors = vec![];
    let mut ix = vec![];
    for ring in 0..=8 {
        let t = ring as f32 / 8.;
        let radius = 0.075 * (1. - t).powf(0.7) * (1. + 0.15 * (t * 20.).sin());
        for j in 0..12 {
            let a = j as f32 * std::f32::consts::TAU / 12.;
            p.push([a.cos() * radius, a.sin() * radius, t]);
            colors.push([1., 0.92 - 0.2 * t, 0.65 - 0.45 * t, (1. - t).powi(2)]);
        }
    }
    for ring in 0..8 {
        for j in 0..12 {
            let a = (ring * 12 + j) as u32;
            let b = (ring * 12 + (j + 1) % 12) as u32;
            ix.extend([a, b, a + 12, b, b + 12, a + 12]);
        }
    }
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    m.insert_indices(Indices::U32(ix));
    m.compute_normals();
    m
}

/// Exact representative render geometry, including main wings and tail fins.
/// Cached once; the BVH rejects distant segments before triangle work.
pub fn cruise_projectile_hit(start:DVec3,end:DVec3)->Option<f64> {
    static SKIN:std::sync::OnceLock<crate::carrier::collider::ProjectileMesh>=std::sync::OnceLock::new();
    SKIN.get_or_init(||crate::carrier::collider::ProjectileMesh::new(&mesh(Kind::AntiShip))).sweep(start,end,0.01)
}
