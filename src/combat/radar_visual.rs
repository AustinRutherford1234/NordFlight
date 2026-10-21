//! Detailed radar-weapon presentation. AIM-120D-inspired dimensions/planform;
//! existing generic and BVR dynamics are deliberately untouched.
use super::*;

#[derive(Resource)]
pub struct Assets {
    pub bodies: [Handle<Mesh>; 2],
    tails: [Handle<Mesh>; 2],
    material: Handle<StandardMaterial>,
}
#[derive(Component)]
pub struct Tail {
    missile: usize,
    quadrant: usize,
}

fn outline(_bvr: bool) -> [(f32, f32); 4] {
    [(0., -0.21), (0.105, -0.09), (0.135, 0.19), (0., 0.24)]
}
fn hinge(quadrant: usize, angle: f32) -> Transform {
    let rotation = Quat::from_rotation_z(
        std::f32::consts::FRAC_PI_4 + quadrant as f32 * std::f32::consts::FRAC_PI_2,
    );
    Transform::from_translation(rotation * Vec3::X * 0.086 + Vec3::Z * 1.48)
        .with_rotation(rotation * Quat::from_rotation_x(angle))
}
pub fn body(bvr: bool, tails: bool) -> Mesh {
    let mut p = Vec::new();
    let mut ix = Vec::new();
    let mut colors = Vec::new();
    let ceramic = [0.82, 0.80, 0.72, 1.];
    let skin = [0.61, 0.64, 0.63, 1.];
    let seam = [0.24, 0.26, 0.25, 1.];
    let metal = [0.37, 0.39, 0.36, 1.];
    let rings = [
        (-1.83, 0.002, ceramic),
        (-1.77, 0.025, ceramic),
        (-1.65, 0.050, ceramic),
        (-1.49, 0.072, ceramic),
        (-1.31, 0.086, ceramic),
        (-1.25, 0.0889, ceramic),
        (-1.24, 0.0892, seam),
        (-1.225, 0.0889, skin),
        (-0.88, 0.0889, skin),
        (-0.87, 0.0891, seam),
        (-0.86, 0.0889, skin),
        (-0.72, 0.0889, skin),
        (-0.715, 0.089, [0.73, 0.53, 0.06, 1.]),
        (-0.68, 0.089, [0.73, 0.53, 0.06, 1.]),
        (-0.675, 0.0889, skin),
        (0.53, 0.0889, skin),
        (0.535, 0.089, [0.30, 0.15, 0.075, 1.]),
        (0.57, 0.089, [0.30, 0.15, 0.075, 1.]),
        (0.575, 0.0889, skin),
        (1.16, 0.0889, skin),
        (1.17, 0.0892, seam),
        (1.185, 0.0889, metal),
        (1.70, 0.0889, metal),
        (1.81, 0.081, metal),
        (1.83, 0.073, metal),
        (1.83, 0.060, seam),
        (1.70, 0.047, [0.08, 0.07, 0.055, 1.]),
        (1.67, 0.002, [0.035, 0.03, 0.025, 1.]),
    ];
    let sides = 64;
    for (z, r, c) in rings {
        for j in 0..sides {
            let a = j as f32 * std::f32::consts::TAU / sides as f32;
            p.push(Vec3::new(a.cos() * r, a.sin() * r, z));
            let grain = 1. - 0.025 * (a * 11. + z * 31.).sin().abs();
            colors.push([c[0] * grain, c[1] * grain, c[2] * grain, 1.]);
        }
    }
    for i in 0..rings.len() - 1 {
        for j in 0..sides {
            let a = (i * sides + j) as u32;
            let b = (i * sides + (j + 1) % sides) as u32;
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
    for (n, reverse) in [(0, true), (((rings.len() - 1) * sides) as u32, false)] {
        for j in 1..sides as u32 - 1 {
            ix.extend(if reverse {
                [n, n + j + 1, n + j]
            } else {
                [n, n + j, n + j + 1]
            });
        }
    }
    for q in 0..4 {
        let rotation = hinge(q, 0.).rotation;
        // AIM-120C/D clipped wings; no early A/B broad-span configuration.
        let span = 0.2235;
        aim9x::fin(
            &mut p,
            &mut ix,
            &[(0.087, -0.05), (span, 0.34), (span, 0.62), (0.087, 0.62)],
            0.004,
            Transform::from_rotation(rotation),
        );
        if tails {
            aim9x::fin(&mut p, &mut ix, &outline(bvr), 0.004, hinge(q, 0.));
        }
        // Flush rectangular proximity-sensor window ahead of wing roots.
        let begin = p.len();
        aim9x::fin(
            &mut p,
            &mut ix,
            &[
                (0.0893, -0.99),
                (0.0893, -0.91),
                (0.091, -0.91),
                (0.091, -0.99),
            ],
            0.012,
            Transform::from_rotation(rotation),
        );
        colors.resize(begin, skin);
        colors.resize(p.len(), [0.06, 0.075, 0.08, 1.]);
    }
    let rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    aim9x::fin(
        &mut p,
        &mut ix,
        &[(0.087, -1.1), (0.097, -1.01), (0.097, 1.38), (0.087, 1.45)],
        0.011,
        Transform::from_rotation(rotation),
    );
    colors.resize(p.len(), skin);
    let mut m = super::mesh(p, ix);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    m
}
pub fn prepare(
    meshes: &mut bevy::prelude::Assets<Mesh>,
    materials: &mut bevy::prelude::Assets<StandardMaterial>,
) -> Assets {
    let tail = {
        let mut p = Vec::new();
        let mut ix = Vec::new();
        aim9x::fin(
            &mut p,
            &mut ix,
            &outline(false),
            0.004,
            Transform::default(),
        );
        meshes.add(super::mesh(p, ix))
    };
    let body = meshes.add(body(false,false));
    Assets {
        bodies: [body.clone(),body],
        tails: [tail.clone(),tail],
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.54, 0.57, 0.56),
            metallic: 0.50,
            perceptual_roughness: 0.42,
            cull_mode: None,
            ..default()
        }),
    }
}
pub fn install(c: &mut Commands, root: Entity, missile: usize, assets: &Assets) {
    for quadrant in 0..4 {
        let e = c
            .spawn((
                Tail { missile, quadrant },
                Mesh3d(assets.tails[0].clone()),
                MeshMaterial3d(assets.material.clone()),
                hinge(quadrant, 0.),
                Visibility::Hidden,
            ))
            .id();
        c.entity(root).add_child(e);
    }
}
pub fn animate(
    combat: Res<Combat>,
    assets: Res<Assets>,
    mut tails: Query<(&Tail, &mut Transform, &mut Visibility, &mut Mesh3d), (Without<aim9x::Actuator>,Without<aim9x::Finish>,super::MechanismFilter)>,
) {
    for (part, mut t, mut v, mut mesh) in &mut tails {
        let m = combat
            .render_missile(part.missile)
            .filter(|m| combat.active && m.alive && m.kind == Weapon::Radar);
        v.set_if_neq(if m.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if let Some(m) = m {
            let handle = &assets.tails[usize::from(m.gnc.long_range)];
            if mesh.0 != *handle {
                mesh.0 = handle.clone();
            }
            let control = aim9x::local_control(m);
            // X-layout: resolve pitch/yaw demand along each physical shaft.
            let control = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_4) * control;
            t.set_if_neq(hinge(
                part.quadrant,
                if m.age > 0.22 {
                    aim9x::fin_angle(control, part.quadrant)
                } else {
                    0.
                },
            ));
        }
    }
}
pub fn plume(m: &Missile, layer: usize) -> Transform {
    // Fixed rocket nozzle: no fictitious radar-missile thrust vectoring.
    let ignition = ((m.age - 0.18) / 0.12).clamp(0., 1.) as f32;
    let power = (m.gnc.thrust / 23000.).clamp(0.1, 1.5) as f32;
    let density = atmosphere(m.position.length() - RADIUS).2.max(0.06);
    let expansion = (1.225 / density).powf(0.16) as f32;
    let pulse = 1. + 0.03 * (m.age as f32 * 73.).sin();
    Transform::from_xyz(0., 0., 1.77).with_scale(Vec3::new(
        [1.6, 2.5, 4.0][layer] * ignition * expansion,
        [1.6, 2.5, 4.0][layer] * ignition * expansion,
        [2.5, 3.5, 4.6][layer] * ignition * power.sqrt() * pulse,
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn radar_plume_stays_axial_and_reflects_motor_output() {
        let f = super::super::flight();
        let mut c = Combat::default();
        c.start(&f, 1);
        c.weapon = Weapon::Radar;
        c.lock = Weapon::Radar.lock_time();
        assert!(c.fire(&f));
        let mut m = c.missiles.remove(0);
        m.age = 1.;
        m.gnc.thrust = 27000.;
        m.gnc.fins = DVec3::X;
        let boost = plume(&m, 0);
        assert_eq!(boost.rotation, Quat::IDENTITY);
        assert!(boost.translation.z < 1.83 && boost.translation.z > 1.7);
        m.gnc.thrust = 5500.;
        let sustain = plume(&m, 0);
        assert!(sustain.scale.z < boost.scale.z);
        assert_eq!(sustain.rotation, Quat::IDENTITY);
        assert_eq!(plume(&m, 0), sustain);
    }
    #[test]
    fn radar_profiles_share_the_aim120d_clipped_planform() {
        let mut counts = Vec::new();
        for bvr in [false, true] {
            let m = body(bvr, true);
            let bevy::mesh::VertexAttributeValues::Float32x3(p) =
                m.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
            else {
                panic!()
            };
            assert!(p.len() > 1800);
            assert!(p.iter().flatten().all(|v| v.is_finite()));
            let low = p.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min);
            let high = p.iter().map(|p| p[2]).fold(f32::NEG_INFINITY, f32::max);
            assert!((high - low - 3.66).abs() < 1e-5);
            assert!(m.indices().unwrap().iter().all(|i| i < p.len()));
            counts.push(
                p.iter()
                    .map(|p| p[0].abs().max(p[1].abs()))
                    .fold(0_f32, f32::max),
            );
        }
        assert_eq!(counts[0], counts[1]);
    }
    #[test]
    fn radar_tail_hinges_are_fixed_and_normalized() {
        for q in 0..4 {
            let a = hinge(q, -0.35);
            let b = hinge(q, 0.35);
            assert_eq!(a.translation, b.translation);
            assert!((a.rotation.length() - 1.).abs() < 1e-6);
            assert_ne!(a.rotation, b.rotation);
        }
    }
}
