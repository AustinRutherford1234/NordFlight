//! Batched exterior hardware. Geometry is built once and shared with stage debris.
use crate::{lunar::profile, space::Stage};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, VertexAttributeValues},
    prelude::*,
    render::render_resource::PrimitiveTopology,
};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

#[derive(Default)]
struct Batch {
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
}
impl Batch {
    fn push(&mut self, mesh: Mesh, t: Transform) {
        let Some(VertexAttributeValues::Float32x3(p)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            return;
        };
        let base = self.positions.len() as u32;
        self.positions.extend(
            p.iter()
                .map(|v| t.transform_point(Vec3::from(*v)).to_array()),
        );
        match mesh.indices() {
            Some(Indices::U32(i)) => self.indices.extend(i.iter().map(|i| base + i)),
            Some(Indices::U16(i)) => self.indices.extend(i.iter().map(|i| base + *i as u32)),
            None => self.indices.extend(base..self.positions.len() as u32),
        }
    }
    fn bar(&mut self, a: Vec3, b: Vec3, radius: f32) {
        let d = b - a;
        self.push(
            Cylinder::new(radius, d.length())
                .mesh()
                .resolution(12)
                .build(),
            Transform::from_translation((a + b) * 0.5)
                .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize())),
        );
    }
    fn ring(&mut self, radius: f32, thickness: f32, z: f32) {
        self.push(
            Torus::new(radius - thickness, radius + thickness)
                .mesh()
                .major_resolution(96)
                .minor_resolution(8)
                .build(),
            Transform::from_xyz(0., 0., z).with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
        );
    }
    fn finish(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh.compute_smooth_normals();
        mesh
    }
}
fn radial(a: f32, r: f32, z: f32) -> Vec3 {
    Vec3::new(a.cos() * r, a.sin() * r, z)
}
pub fn window_frame(window: &Mesh) -> Mesh {
    let Some(VertexAttributeValues::Float32x3(p)) = window.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        unreachable!()
    };
    let Some(Indices::U32(indices)) = window.indices() else {
        unreachable!()
    };
    let mut edges = std::collections::BTreeMap::new();
    for t in indices.chunks_exact(3) {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    let mut batch = Batch::default();
    for ((a, b), count) in edges {
        if count == 1 {
            batch.bar(
                Vec3::from(p[a as usize]) * 1.002,
                Vec3::from(p[b as usize]) * 1.002,
                0.026,
            );
        }
    }
    batch.finish()
}
pub fn strut(length: f32, radius: f32) -> Mesh {
    let mut b = Batch::default();
    b.push(
        profile(
            &[
                (-length * 0.5, radius * 0.68),
                (-length * 0.42, radius * 0.68),
                (-length * 0.42, radius * 1.28),
                (-length * 0.38, radius * 1.28),
                (-length * 0.38, radius),
                (length * 0.22, radius),
                (length * 0.22, radius * 1.30),
                (length * 0.26, radius * 1.30),
                (length * 0.26, radius * 0.66),
                (length * 0.5, radius * 0.66),
            ],
            24,
        ),
        Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
    );
    for y in [-length * 0.48, length * 0.48] {
        b.push(
            Sphere::new(radius * 1.45).mesh().uv(16, 10),
            Transform::from_xyz(0., y, 0.),
        );
    }
    b.finish()
}
pub fn footpad() -> Mesh {
    let mut b = Batch::default();
    b.push(
        profile(
            &[
                (-0.09, 0.01),
                (-0.09, 0.54),
                (-0.04, 0.73),
                (0.05, 0.73),
                (0.10, 0.60),
                (0.10, 0.01),
            ],
            64,
        ),
        Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
    );
    for i in 0..8 {
        let a = i as f32 * TAU / 8.;
        b.bar(
            Vec3::new(0., 0.11, 0.),
            Vec3::new(a.cos() * 0.57, 0.06, a.sin() * 0.57),
            0.025,
        );
    }
    b.finish()
}
fn arc_panel(radius: f32, a: f32, width: f32, z0: f32, z1: f32) -> Mesh {
    let mut batch = Batch::default();
    for i in 0..16 {
        let aa = a - width * 0.5 + width * i as f32 / 16.;
        let bb = aa + width / 16.;
        let base = batch.positions.len() as u32;
        batch.positions.extend(
            [
                radial(aa, radius, z0),
                radial(bb, radius, z0),
                radial(aa, radius, z1),
                radial(bb, radius, z1),
            ]
            .map(|v| v.to_array()),
        );
        batch
            .indices
            .extend([base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }
    batch.finish()
}
fn foil_panel(a: f32, side: usize, row: usize) -> Mesh {
    // Each blanket is bounded by the octagonal pressure structure. Small folds
    // change the actual normal and specular response without floating sheets.
    let n = Vec3::new(a.cos(), a.sin(), 0.);
    let tangent = Vec3::new(-a.sin(), a.cos(), 0.);
    let apothem = 3.7 * (PI / 8.).cos() + 0.012;
    let mut b = Batch::default();
    for j in 0..=6 {
        for i in 0..=8 {
            let u = -1.49 + 2.98 * i as f32 / 8.;
            let z = 4.78 + row as f32 * 1.10 + j as f32 * 1.10 / 6.;
            let edge = i == 0 || i == 8 || j == 0 || j == 6;
            let fold = if edge {
                0.
            } else {
                0.022 * ((i * 17 + j * 31 + side * 7) as f32).sin()
            };
            b.positions
                .push((n * (apothem + fold) + tangent * u + Vec3::Z * z).to_array());
        }
    }
    for j in 0..6 {
        for i in 0..8 {
            let k = (j * 9 + i) as u32;
            b.indices.extend([k, k + 1, k + 9, k + 1, k + 10, k + 9]);
        }
    }
    b.finish()
}

/// Material-indexed batches: ceramic, titanium, dark hardware, gold and light foil.
/// None is capsule hardware; all other batches detach with their own stage.
pub fn build() -> Vec<(Option<Stage>, usize, Mesh)> {
    let mut result = Vec::new();
    for stage in [
        None,
        Some(Stage::Return),
        Some(Stage::Descent),
        Some(Stage::Transfer),
    ] {
        let mut b: [Batch; 5] = std::array::from_fn(|_| Batch::default());
        match stage {
            None => {
                // Pressure-shell seams and recessed heat-shield attachment rim.
                for (z, r) in [
                    (-3.8, 0.525),
                    (-3.1, 0.855),
                    (-0.8, 1.727),
                    (1.06, 2.107),
                    (1.43, 2.145),
                ] {
                    b[1].ring(r, 0.014, z);
                }
                b[1].push(
                    profile(&[(-4.2, 0.01), (-4.2, 0.32), (-4.15, 0.36)], 64),
                    Transform::default(),
                );
                b[2].ring(0.27, 0.025, -4.21);
                for i in 0..32 {
                    let a = i as f32 * TAU / 32.;
                    b[1].push(
                        Sphere::new(0.027).mesh().uv(8, 6),
                        Transform::from_translation(radial(a, 2.125, 1.27)),
                    );
                }
                // Hatch conforms to the lower cone, away from the pilot glazing.
                let a = -FRAC_PI_2;
                for da in [-0.27, 0.27] {
                    b[1].bar(
                        radial(a + da, 1.31, -1.95),
                        radial(a + da, 1.96, 0.35),
                        0.026,
                    );
                }
                for (z, r) in [(-1.95, 1.31), (0.35, 1.96)] {
                    b[1].push(
                        arc_panel(r, a, 0.54, z - 0.025, z + 0.025),
                        Transform::default(),
                    );
                }
                b[1].bar(
                    Vec3::new(-0.12, -1.70, -0.75),
                    Vec3::new(0.12, -1.70, -0.75),
                    0.035,
                );
                // Thermal-shield tile joints stay on the actual curved shield.
                for (r, z) in [(0.45, 1.704), (0.9, 1.685), (1.35, 1.665), (1.8, 1.647)] {
                    b[2].ring(r, 0.009, z);
                }
                for i in 0..12 {
                    let a = i as f32 * TAU / 12.;
                    b[2].bar(radial(a, 0.18, 1.716), radial(a, 1.87, 1.645), 0.008);
                }
                // Collar and tiny vent recesses behind the capsule glazing.
                for i in 0..12 {
                    let a = i as f32 * TAU / 12.;
                    b[2].push(arc_panel(1.94, a, 0.09, 0.55, 0.76), Transform::default());
                }
            }
            Some(Stage::Return) => {
                for z in [1.82, 2.03, 3.96, 4.13] {
                    b[1].ring(1.914, 0.028, z);
                }
                for i in 0..12 {
                    let a = i as f32 * TAU / 12.;
                    b[if i % 3 == 0 { 2 } else { 0 }]
                        .push(arc_panel(1.922, a, 0.46, 2.12, 3.85), Transform::default());
                    b[1].bar(
                        radial(a + 0.255, 1.934, 2.07),
                        radial(a + 0.255, 1.934, 3.94),
                        0.016,
                    );
                    if i % 3 == 0 {
                        for k in 0..12 {
                            let z = 2.18 + k as f32 * 0.133;
                            b[1].push(
                                arc_panel(1.935, a, 0.43, z, z + 0.025),
                                Transform::default(),
                            );
                        }
                    }
                }
                // Engine mounting plate, gimbal braces and regenerative channels.
                b[2].push(
                    profile(&[(4.22, 0.3), (4.22, 1.27), (4.27, 1.27)], 64),
                    Transform::default(),
                );
                for i in 0..8 {
                    let a = i as f32 * TAU / 8.;
                    b[1].bar(radial(a, 1.12, 4.23), radial(a, 0.39, 4.64), 0.045);
                }
                for i in 0..18 {
                    let z = 4.65 + i as f32 * 1.43 / 18.;
                    let r = if z < 5.5 {
                        0.35 + (z - 4.6) * 0.33 / 0.9
                    } else {
                        0.68 + (z - 5.5) * 0.27 / 0.6
                    };
                    b[1].ring(r + 0.008, 0.012, z);
                }
                b[1].ring(0.95, 0.035, 6.1);
                // Four small service RCS quads with open bell mouths.
                for i in 0..4 {
                    let a = PI / 4. + i as f32 * FRAC_PI_2;
                    let p = radial(a, 1.98, 2.32);
                    b[1].push(
                        Cuboid::new(0.32, 0.30, 0.48).mesh().build(),
                        Transform::from_translation(p).with_rotation(Quat::from_rotation_z(a)),
                    );
                    for dz in [-0.14, 0.14] {
                        let r = Quat::from_rotation_arc(Vec3::Z, radial(a, 1., 0.));
                        b[2].push(
                            profile(&[(0., 0.048), (0.12, 0.072), (0.22, 0.105)], 24),
                            Transform::from_translation(p + Vec3::Z * dz).with_rotation(r),
                        );
                    }
                }
            }
            Some(Stage::Descent) => {
                for i in 0..8 {
                    let a = PI / 8. + i as f32 * TAU / 8.;
                    for row in 0..2 {
                        b[if (i + row) % 3 == 0 { 4 } else { 3 }]
                            .push(foil_panel(a, i, row), Transform::default());
                    }
                    let corner = i as f32 * TAU / 8.;
                    b[2].bar(
                        radial(corner, 3.715, 4.73),
                        radial(corner, 3.715, 7.13),
                        0.029,
                    );
                    b[1].bar(radial(a, 1.92, 4.09), radial(a, 3.28, 4.66), 0.065);
                    // Small louvered equipment panels inset against each face.
                    b[2].push(
                        arc_panel(3.72 * (PI / 8.).cos() + 0.035, a, 0.18, 5.35, 5.89),
                        Transform::default(),
                    );
                }
                for (x, y) in [(-2., -2.), (-2., 2.), (2., -2.), (2., 2.)] {
                    for k in 0..9 {
                        let z = 6.98 + k as f32 * 0.075;
                        let r = if z < 7.4 {
                            0.28 + (z - 6.9) * 0.35 / 0.5
                        } else {
                            0.63 + (z - 7.4) * 0.09 / 0.3
                        };
                        b[1].push(
                            Torus::new(r - 0.009, r + 0.009)
                                .mesh()
                                .major_resolution(32)
                                .minor_resolution(6)
                                .build(),
                            Transform::from_xyz(x, y, z)
                                .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        );
                    }
                    b[1].push(
                        Torus::new(0.69, 0.735)
                            .mesh()
                            .major_resolution(48)
                            .minor_resolution(8)
                            .build(),
                        Transform::from_xyz(x, y, 7.7)
                            .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                    );
                }
                // Strut clevises are attached to the stage, not floating at the feet.
                for x in [-2.5675, 2.5675] {
                    for y in [-2.5675, 2.5675] {
                        b[1].push(
                            Sphere::new(0.19).mesh().uv(16, 12),
                            Transform::from_xyz(x, y, 5.1),
                        );
                    }
                }
            }
            Some(Stage::Transfer) => {
                // Open structural adapter closes the former empty interstage gap.
                for i in 0..16 {
                    let a = i as f32 * TAU / 16.;
                    b[1].bar(radial(a, 3.45, 7.18), radial(a, 3.2, 9.22), 0.065);
                    if i % 2 == 0 {
                        b[1].bar(
                            radial(a, 3.45, 7.18),
                            radial(a + TAU / 16., 3.2, 9.22),
                            0.036,
                        );
                    }
                }
                for z in [8.72, 9.23, 11.16, 15.43, 17.13, 17.7] {
                    b[1].ring(if z < 9. { 2.73 } else { 3.215 }, 0.035, z);
                }
                // Corrugated tank skirts, dark roll-pattern blocks and feed lines.
                for i in 0..72 {
                    let a = i as f32 * TAU / 72.;
                    for (z0, z1) in [(9.25, 10.1), (16.65, 17.48)] {
                        b[1].bar(radial(a, 3.216, z0), radial(a, 3.216, z1), 0.018);
                    }
                }
                for i in 0..8 {
                    let a = i as f32 * TAU / 8.;
                    b[2].push(arc_panel(3.218, a, 0.24, 10.2, 11.13), Transform::default());
                    b[2].push(
                        arc_panel(3.218, a, 0.24, 15.48, 16.54),
                        Transform::default(),
                    );
                }
                for a in [0.3, PI + 0.3] {
                    b[1].bar(radial(a, 3.23, 10.1), radial(a, 3.23, 16.6), 0.05);
                }
                b[2].push(
                    profile(&[(18.18, 0.62), (18.18, 2.47), (18.25, 2.47)], 64),
                    Transform::default(),
                );
                for i in 0..12 {
                    let a = i as f32 * TAU / 12.;
                    b[1].bar(radial(a, 1.65, 18.22), radial(a, 0.67, 18.65), 0.05);
                }
                for i in 0..24 {
                    let z = 18.55 + i as f32 * 2.02 / 24.;
                    let r = if z < 19.3 {
                        0.63 + (z - 18.5) * 0.52 / 0.8
                    } else {
                        1.15 + (z - 19.3) * 0.50 / 1.3
                    };
                    b[1].ring(r + 0.008, 0.015, z);
                }
                b[1].ring(1.65, 0.04, 20.6);
            }
            _ => unreachable!(),
        }
        for (material, batch) in b.into_iter().enumerate() {
            if !batch.positions.is_empty() {
                result.push((stage, material, batch.finish()));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn batched_exterior_is_finite_bounded_and_attached_to_each_stage() {
        let batches = build();
        assert!(batches.len() <= 20);
        for stage in [
            None,
            Some(Stage::Return),
            Some(Stage::Descent),
            Some(Stage::Transfer),
        ] {
            assert!(batches.iter().any(|b| b.0 == stage));
        }
        for (_, _, mesh) in &batches {
            let Some(VertexAttributeValues::Float32x3(p)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!()
            };
            let Some(VertexAttributeValues::Float32x3(n)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
            else {
                panic!()
            };
            assert_eq!(p.len(), n.len());
            assert!(
                p.iter()
                    .all(|p| Vec3::from(*p).is_finite() && Vec3::from(*p).length() < 22.)
            );
            assert!(n.iter().all(|n| Vec3::from(*n).is_finite()));
            if let Some(Indices::U32(indices)) = mesh.indices() {
                assert!(indices.iter().all(|i| (*i as usize) < p.len()));
            }
        }
    }
}
