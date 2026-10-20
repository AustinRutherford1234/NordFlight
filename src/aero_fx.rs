//! Original procedural atmospheric effects, inspired by Firefly's visual language.
use crate::{ViewState, physics::*, world::local};
use bevy::{
    asset::RenderAssetUsages,
    math::DVec3,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
#[derive(Component)]
pub struct Effect {
    vehicle: Vehicle,
    kind: u8,
    index: u8,
}
pub fn strengths(f: &Flight) -> (f32, f32, f32) {
    if f.crashed || f.density < 1e-8 {
        return (0.0, 0.0, 0.0);
    }
    let q = 0.5 * f.density * f.airspeed.powi(2);
    // Heating proxy has the density-root / speed-cubed dependence of stagnation heating.
    let heat = ((f.density / 0.001).sqrt() * (f.airspeed / 6500.0).powi(3) - 0.12).clamp(0.0, 1.0);
    let cone = (-((f.mach - 1.02) / 0.16).powi(2)).exp()
        * (q / 18000.0).clamp(0.0, 1.0)
        * (f.density / 0.4).clamp(0.0, 1.0);
    let vapor = ((f.g_load.abs() - 1.2) / 4.0).clamp(0.0, 1.0)
        * (q / 15000.0).clamp(0.0, 1.0)
        * (f.density / 0.6).clamp(0.0, 1.0);
    (heat as f32, cone as f32, vapor as f32)
}
fn cone() -> Mesh {
    let mut p = Vec::new();
    let mut col = Vec::new();
    let mut idx = Vec::new();
    for j in 0..=12 {
        let t = j as f32 / 12.0;
        for i in 0..=48 {
            let a = i as f32 / 48.0 * std::f32::consts::TAU;
            let r = 0.12 + t * 3.8;
            p.push([r * a.cos(), r * a.sin(), t * 4.5]);
            col.push([
                1.0,
                1.0,
                1.0,
                (std::f32::consts::PI * t).sin().powi(2) * 0.25,
            ]);
        }
    }
    for j in 0..12 {
        for i in 0..48 {
            let a = (j * 49 + i) as u32;
            let b = a + 49;
            idx.extend([a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; 13 * 49]);
    m.insert_indices(Indices::U32(idx));
    m.compute_smooth_normals();
    m
}
pub fn setup(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let sphere = meshes.add(Sphere::new(1.0).mesh().uv(20, 12));
    let cone = meshes.add(cone());
    let strand = meshes.add(Capsule3d::new(0.07, 1.0));
    for vehicle in [Vehicle::Gripen, Vehicle::Spacecraft] {
        for kind in 0..3 {
            let count = match kind {
                0 => 25,
                1 => 1,
                _ => 2,
            };
            for index in 0..count {
                let material = materials.add(StandardMaterial {
                    base_color: Color::NONE,
                    emissive: LinearRgba::BLACK,
                    alpha_mode: AlphaMode::Add,
                    unlit: true,
                    cull_mode: None,
                    ..default()
                });
                c.spawn((
                    Mesh3d(if kind == 1 {
                        cone.clone()
                    } else if kind == 2 {
                        strand.clone()
                    } else {
                        sphere.clone()
                    }),
                    MeshMaterial3d(material),
                    Transform::default(),
                    Visibility::Hidden,
                    Effect {
                        vehicle,
                        kind,
                        index,
                    },
                    bevy::light::NotShadowCaster,
                    bevy::light::NotShadowReceiver,
                    bevy::camera::visibility::NoFrustumCulling,
                ));
            }
        }
    }
}
pub fn update(
    f: Res<Flight>,
    view: Res<ViewState>,
    time: Res<Time>,
    mut effects: Query<(
        &Effect,
        &MeshMaterial3d<StandardMaterial>,
        &mut Transform,
        &mut Visibility,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let (heat, cone, vapor) = strengths(&f);
    let air = f.velocity - f.wind;
    let downstream = -air.normalize_or_zero();
    let flow = (crate::world::frame(&f).conjugate() * downstream).as_vec3();
    let rotation = Quat::from_rotation_arc(Vec3::Z, flow);
    let center = local(f.position, &f);
    let phase = if f.paused {
        f.elapsed as f32
    } else {
        time.elapsed_secs()
    };
    for (effect, handle, mut t, mut vis) in &mut effects {
        let strength = match effect.kind {
            0 => heat,
            1 => cone,
            _ => vapor,
        };
        let active =
            effect.vehicle == f.vehicle && !view.map && view.index != 3 && strength > 0.015;
        *vis = if active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if !active {
            continue;
        }
        let Some(mat) = materials.get_mut(&handle.0) else {
            continue;
        };
        match effect.kind {
            0 => {
                let i = effect.index as f32;
                let angle = i * 2.39996 + phase * 0.55;
                let seed = (phase * 3.0 + i * 0.618).fract();
                let radial = Vec3::new(angle.cos(), angle.sin(), 0.0);
                let width = if f.vehicle == Vehicle::Gripen {
                    1.2
                } else {
                    1.6
                };
                if effect.index == 0 {
                    let front = if f.vehicle == Vehicle::Spacecraft
                        && f.space.stage == crate::space::Stage::Capsule
                    {
                        1.8
                    } else {
                        5.
                    };
                    t.translation = center + flow * (-front);
                    t.rotation = rotation;
                    t.scale = Vec3::new(width, width, 0.26);
                } else {
                    t.translation = center
                        + rotation
                            * (radial * (width + seed * 1.0) + Vec3::Z * (-4.5 + seed * 18.0));
                    t.rotation = rotation;
                    t.scale = Vec3::new(0.08 + seed * 0.08, 0.08 + seed * 0.08, 0.8 + seed * 2.5);
                }
                let alpha = strength * (1.0 - seed) * 0.38;
                mat.base_color = Color::srgba(1.0, 0.22 + seed * 0.35, 0.055, alpha);
                mat.emissive = LinearRgba::new(3.0 * alpha, 0.6 * alpha, 0.1 * alpha, 1.0);
            }
            1 => {
                t.translation = center + rotation * Vec3::Z * (-1.0);
                t.rotation = rotation;
                t.scale = Vec3::ONE;
                mat.base_color = Color::srgba(0.7, 0.85, 1.0, strength * 0.8);
                mat.emissive = LinearRgba::BLACK;
            }
            _ => {
                let side = if effect.index == 0 { -1.0 } else { 1.0 };
                let p = f.orientation * DVec3::new(side * 4.1, -0.12, 2.2);
                t.translation = local(f.position + p, &f) + flow * 2.0;
                t.rotation = Quat::from_rotation_arc(Vec3::Y, flow);
                t.scale = Vec3::new(1.4, 4.0, 1.4);
                mat.base_color = Color::srgba(0.65, 0.8, 1.0, strength * 0.25);
                mat.emissive = LinearRgba::BLACK;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effects_require_air_and_matching_flight_conditions() {
        let mut f = Flight::spacecraft();
        assert_eq!(strengths(&f), (0.0, 0.0, 0.0));
        f.density = 0.001;
        f.airspeed = 7500.0;
        f.mach = 25.0;
        assert!(strengths(&f).0 > 0.8);
        f.density = 1.0;
        f.airspeed = 340.0;
        f.mach = 1.0;
        f.g_load = 4.0;
        let s = strengths(&f);
        assert!(s.1 > 0.8 && s.2 > 0.0);
        f.mach = 0.3;
        assert!(strengths(&f).1 < 0.01);
    }
}
