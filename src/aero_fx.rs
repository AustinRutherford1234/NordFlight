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
    let cone = vapor_cone_strength(f.mach, f.density, q);
    let vapor = ((f.g_load.abs() - 1.2) / 4.0).clamp(0.0, 1.0)
        * (q / 15000.0).clamp(0.0, 1.0)
        * (f.density / 0.6).clamp(0.0, 1.0);
    (heat as f32, cone as f32, vapor as f32)
}
/// Transonic condensation (Prandtl-Glauert) cloud: it forms only in a narrow
/// band around Mach 1, where expansion over the airframe drops the local
/// temperature below the dew point, then vanishes once the flow is fully
/// supersonic. Dense, low-altitude air carries the moisture that makes it.
pub fn vapor_cone_strength(mach: f64, density: f64, q: f64) -> f64 {
    (-((mach - 0.985) / 0.04).powi(2)).exp()
        * (q / 18000.0).clamp(0.0, 1.0)
        * (density / 0.55).clamp(0.0, 1.0)
}
/// Half-angle of the shroud from the flight axis. As Mach rises through the
/// transonic band the terminating shock moves aft and leans back, so the
/// cloud narrows from a broad bell (~80 deg at M 0.93) toward the Mach angle
/// asin(1/M), which it follows once supersonic.
pub fn vapor_cone_half_angle(mach: f64) -> f64 {
    let t = ((mach - 0.93) / 0.12).clamp(0.0, 1.0);
    let bell = 80f64.to_radians() + (58f64.to_radians() - 80f64.to_radians()) * t;
    if mach > 1.0 {
        bell.min((1.0 / mach).asin())
    } else {
        bell
    }
}
/// Unit shroud: rings from radius 1 (the airframe) to radius 1 + reach, with
/// axial position equal to radial distance (a 45 degree cone opening
/// downstream along +Z). Scaling Z by cot(half-angle) sets the real angle.
/// Opacity rises quickly off the skin and feathers out at the rim, with fixed
/// mottling so it reads as condensed vapour rather than a solid funnel.
fn cone() -> Mesh {
    let (rings, segments) = (14, 72);
    let mut p = Vec::new();
    let mut col = Vec::new();
    let mut idx = Vec::new();
    // Three nested shells a little apart along the flow give the cloud depth
    // from every viewing angle instead of a single razor-thin sheet.
    for (layer, (shift, weight)) in [(0.0f32, 0.55f32), (0.22, 0.35), (0.45, 0.22)]
        .into_iter()
        .enumerate()
    {
        let base = (layer * (rings + 1) * (segments + 1)) as u32;
        for j in 0..=rings {
            let t = j as f32 / rings as f32;
            for i in 0..=segments {
                let a = i as f32 / segments as f32 * std::f32::consts::TAU;
                let r = 1.0 + t * 1.6;
                p.push([r * a.cos(), r * a.sin(), r - 1.0 + shift]);
                let inner = (t / 0.18).min(1.0);
                let outer = (1.0 - t).powf(1.6);
                let i_wrap = i % segments;
                let mottle = 0.72
                    + 0.18 * ((i_wrap as f32 * 0.61).sin() * (j as f32 * 1.7 + 0.4).cos())
                    + 0.10 * ((i_wrap as f32 * 2.3 + j as f32).sin());
                col.push([1.0, 1.0, 1.0, inner * outer * mottle * weight]);
            }
        }
        for j in 0..rings {
            for i in 0..segments {
                let a = base + (j * (segments + 1) + i) as u32;
                let b = a + segments as u32 + 1;
                idx.extend([a, a + 1, b, a + 1, b + 1, b]);
            }
        }
    }
    let count = 3 * (rings + 1) * (segments + 1);
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; count]);
    m.insert_indices(Indices::U32(idx));
    m.compute_smooth_normals();
    m
}
// const AI_CONES: u8 = 12;
/// Place a vapor shroud for an aircraft of `aircraft` type at `position`
/// flying with airspeed vector `air`.
fn place_cone(
    t: &mut Transform,
    f: &Flight,
    aircraft: crate::aircraft::Aircraft,
    position: DVec3,
    air: DVec3,
    mach: f64,
) {
    let spec = aircraft.spec();
    let flow = (crate::world::frame(f).conjugate() * -air.normalize_or_zero()).as_vec3();
    // The shroud wraps the fuselage just aft of the wing's thickest section.
    let skin = (spec.span * 0.11) as f32;
    let half_angle = vapor_cone_half_angle(mach) as f32;
    let axial = (1.0 / half_angle.tan()).max(0.03);
    t.translation = local(position, f) + flow * (spec.length * 0.08) as f32;
    t.rotation = Quat::from_rotation_arc(Vec3::Z, flow);
    // Radial reach 1.6 x skin covers roughly the wingspan.
    t.scale = Vec3::new(skin * 1.7, skin * 1.7, skin * 1.7 * axial);
}
fn cone_color(strength: f64) -> Color {
    Color::srgba(0.93, 0.95, 0.98, strength.min(1.0) as f32)
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
        for kind in 0..4 {
            let count = match kind {
                0 => 25,
                // Vapor cone rendering disabled until the shroud looks better.
                // 1 => 1,
                1 => 0,
                2 => 2,
                // Vapor cones for combat aircraft, drawn in either vehicle mode.
                // _ if vehicle == Vehicle::Gripen => AI_CONES,
                _ => 0,
            };
            for index in 0..count {
                let material = materials.add(StandardMaterial {
                    base_color: Color::NONE,
                    emissive: LinearRgba::BLACK,
                    // Condensation is a white cloud that occludes, not a glow.
                    alpha_mode: if kind == 1 || kind == 3 {
                        AlphaMode::Blend
                    } else {
                        AlphaMode::Add
                    },
                    unlit: true,
                    cull_mode: None,
                    ..default()
                });
                c.spawn((
                    Mesh3d(if kind == 1 || kind == 3 {
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
    combat: Option<Res<crate::combat::Combat>>,
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
        if effect.kind == 3 {
            // AI aircraft: same condensation physics from each target's state.
            let target = combat
                .as_ref()
                .filter(|c| c.active)
                .and_then(|c| c.targets.get(effect.index as usize))
                .filter(|t| t.alive);
            let state = target
                .map(|t| {
                    let air = t.velocity - f.wind;
                    let (_, _, rho, sound) = atmosphere(t.position.length() - RADIUS);
                    let speed = air.length();
                    let mach = speed / sound.max(1.0);
                    (
                        t,
                        air,
                        mach,
                        vapor_cone_strength(mach, rho, 0.5 * rho * speed * speed),
                    )
                })
                .filter(|s| s.3 > 0.015 && !view.map);
            vis.set_if_neq(if state.is_some() {
                Visibility::Visible
            } else {
                Visibility::Hidden
            });
            if let Some((target, air, mach, strength)) = state {
                place_cone(
                    &mut t,
                    &f,
                    target.damage.aircraft,
                    target.position,
                    air,
                    mach,
                );
                let color = cone_color(strength);
                if materials
                    .get(&handle.0)
                    .is_some_and(|m| m.base_color != color)
                    && let Some(mat) = materials.get_mut(&handle.0)
                {
                    mat.base_color = color;
                }
            }
            continue;
        }
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
                let _ = (center, rotation);
                place_cone(&mut t, &f, f.aircraft, f.position, air, f.mach);
                mat.base_color = cone_color(strength as f64);
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
        // Fully supersonic flow leaves no condensation shroud.
        f.mach = 1.15;
        assert!(strengths(&f).1 < 0.01);
    }
    #[test]
    fn vapor_cone_angle_sweeps_back_with_mach() {
        let deg = |m: f64| vapor_cone_half_angle(m).to_degrees();
        assert!(deg(0.93) > 79.0);
        assert!(deg(0.97) < deg(0.93) && deg(1.0) < deg(0.97) && deg(1.05) < deg(1.0));
        assert!((deg(2.0) - 30.0).abs() < 0.01);
    }
}
