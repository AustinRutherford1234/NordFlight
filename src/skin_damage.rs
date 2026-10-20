//! Persistent surface-fitted scorch, fragment pits and torn paint. Visual only;
//! component damage and detached-control aerodynamics remain owned by physics.
use crate::{aircraft_detail, combat::Combat, physics::*};
use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};
#[derive(Component)]
pub struct Scar {
    region: usize,
    target: Option<usize>,
}
fn amount(d: &AircraftDamage, region: usize) -> f64 {
    match region {
        0 | 1 => 1. - d.wings[region].min(d.structure[region + 1]),
        2 | 3 => (1. - d.engine).max(1. - d.structure[4]),
        4 | 5 => 1. - d.structure[0].min(d.radar).min(d.avionics),
        _ => (1. - d.structure[3])
            .max(1. - d.hydraulics)
            .max((d.fuel_leak * 0.25).min(0.85)),
    }
    .clamp(0., 1.)
}
fn point(region: usize, u: f32, v: f32, npc: bool) -> Vec3 {
    let side = if region % 2 == 0 { -1. } else { 1. };
    if region < 2 {
        let x = 2.05 + u * 0.8;
        let z = 1.45 + v * 0.6;
        let y = if npc {
            -0.1 - (x - 0.65) * 0.02 / 3.5
        } else {
            crate::world::wing_surface_y(x, z, true)
        };
        return Vec3::new(side * x, y + 0.008, z);
    }
    let z = match region {
        2 | 3 => 4.35,
        4 | 5 => -5.15,
        _ => 0.15,
    } + v * 0.78;
    let angle = if side > 0. {
        0.62 + u * 0.62
    } else {
        std::f32::consts::PI - 0.62 - u * 0.62
    };
    if !npc {
        return aircraft_detail::surface(z, angle) * Vec3::new(1.008, 1.008, 1.);
    }
    let sections = [
        (-7., 0.03, 0.03),
        (-5., 0.42, 0.34),
        (-3., 0.65, 0.57),
        (0., 0.84, 0.66),
        (3.5, 0.72, 0.59),
        (5.6, 0.51, 0.49),
    ];
    let s = sections.windows(2).find(|s| z <= s[1].0).unwrap();
    let t = (z - s[0].0) / (s[1].0 - s[0].0);
    let w = s[0].1 + (s[1].1 - s[0].1) * t;
    let h = s[0].2 + (s[1].2 - s[0].2) * t;
    Vec3::new(w * angle.cos() * 1.012, h * angle.sin() * 1.012, z)
}
fn patch(region: usize, npc: bool) -> Mesh {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let mut fan = |cx: f32, cy: f32, radius: f32, pit: bool| {
        for n in 0..32 {
            let a = n as f32 * std::f32::consts::TAU / 32.;
            let b = (n + 1) as f32 * std::f32::consts::TAU / 32.;
            let edge = |angle: f32| {
                let r = radius
                    * (0.85
                        + 0.10 * (angle * 7. + region as f32).sin()
                        + 0.05 * (angle * 13.).cos());
                point(region, cx + r * angle.cos(), cy + r * angle.sin(), npc)
            };
            let base = positions.len() as u32;
            positions.extend([point(region, cx, cy, npc), edge(a), edge(b)]);
            if pit {
                // Dark puncture with alternating exposed-metal and charred rim.
                let rim = if n % 4 == 0 {
                    [0.38, 0.31, 0.23, 0.95]
                } else {
                    [0.07, 0.055, 0.04, 0.95]
                };
                colors.extend([[0.008, 0.009, 0.012, 1.], rim, rim]);
            } else {
                colors.extend([
                    [0.025, 0.02, 0.015, 0.85],
                    [0.11, 0.075, 0.035, 0.],
                    [0.11, 0.075, 0.035, 0.],
                ]);
            }
            indices.extend([base, base + 1, base + 2]);
        }
    };
    fan(0., 0., 0.95, false);
    for n in 0..11 {
        let a = n as f32 * 2.39996 + region as f32;
        let r = 0.12 + (n % 4) as f32 * 0.15;
        fan(
            r * a.cos(),
            r * a.sin(),
            0.055 + (n % 3) as f32 * 0.025,
            true,
        );
    }
    let count = positions.len();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        positions
            .into_iter()
            .map(|p| p.to_array())
            .collect::<Vec<_>>(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; count]);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();
    mesh
}
pub fn setup(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    root: Entity,
    target: Option<usize>,
) {
    for region in 0..8 {
        let mat = mats.add(StandardMaterial {
            base_color: Color::WHITE,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            perceptual_roughness: 0.95,
            depth_bias: 1.,
            ..default()
        });
        let e = c
            .spawn((
                Mesh3d(meshes.add(patch(region, target.is_some()))),
                MeshMaterial3d(mat),
                Transform::default(),
                Visibility::Hidden,
                Scar { region, target },
                bevy::light::NotShadowCaster,
            ))
            .id();
        c.entity(root).add_child(e);
    }
}
pub fn update(
    f: Res<Flight>,
    combat: Res<Combat>,
    mut scars: Query<(&Scar, &mut Visibility, &MeshMaterial3d<StandardMaterial>)>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    for (scar, mut visibility, material) in &mut scars {
        let d = match scar.target {
            Some(i) => combat
                .targets
                .get(i)
                .filter(|t| combat.active && t.alive)
                .map(|t| &t.damage),
            None => (f.vehicle == Vehicle::Gripen && !f.crashed).then_some(&f.damage),
        };
        let damage = d.map_or(0., |d| amount(d, scar.region)) as f32;
        visibility.set_if_neq(if damage > 0.025 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if damage > 0.025 {
            crate::render_cache::optics(
                &mut mats,
                &material.0,
                Color::srgba(1., 1., 1., (damage * 1.7).min(1.)),
                LinearRgba::BLACK,
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn damage_marks_follow_component_state_and_reset() {
        let mut d = AircraftDamage::default();
        assert!((0..8).all(|i| amount(&d, i) == 0.));
        d.wings[0] = 0.4;
        assert!(amount(&d, 0) > 0.5);
        assert_eq!(amount(&d, 1), 0.);
        d.engine = 0.;
        assert_eq!(amount(&d, 2), 1.);
        d = AircraftDamage::default();
        assert_eq!(amount(&d, 2), 0.);
    }
    #[test]
    fn surface_fitted_damage_geometry_is_finite() {
        for npc in [false, true] {
            for region in 0..8 {
                let m = patch(region, npc);
                assert!(m.count_vertices() > 1000);
                if let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
                    m.attribute(Mesh::ATTRIBUTE_POSITION)
                {
                    assert!(p.iter().flatten().all(|v| v.is_finite()));
                }
            }
        }
    }
}
