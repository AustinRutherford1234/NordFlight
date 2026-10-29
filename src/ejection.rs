//! Game approximation of a two-seat escape sequence, independent of render time.
use crate::physics::*;
use bevy::{
    math::{DQuat, DVec3},
    prelude::*,
};
#[derive(Clone, Debug, Default)]
pub struct Body {
    pub position: DVec3,
    pub velocity: DVec3,
    pub grounded: bool,
}
#[derive(Clone, Debug, Default)]
pub struct State {
    pub active: bool,
    pub age: f64,
    pub attitude: DQuat,
    pub pilots: [Body; 2],
    pub seats: [Body; 2],
    pub canopy: Body,
}
pub fn delay(i: usize) -> f64 {
    if i == 1 { 0.30 } else { 0.55 }
}
pub fn seat_offset(i: usize) -> DVec3 {
    crate::aircraft::Aircraft::Hornet.cabin_offset().as_dvec3()
        + DVec3::new(0., 0.40 + i as f64 * 0.12, -2.75 + i as f64 * 1.45)
}
pub fn start(f: &mut Flight) -> bool {
    if f.aircraft != crate::aircraft::Aircraft::Hornet
        || f.vehicle != Vehicle::Gripen
        || f.crashed
        || f.ejection.active
    {
        return false;
    }
    let bodies = std::array::from_fn(|i| Body {
        position: f.position + f.orientation * seat_offset(i),
        velocity: f.velocity,
        grounded: false,
    });
    f.ejection = State {
        active: true,
        age: 0.,
        attitude: f.orientation,
        pilots: bodies.clone(),
        seats: bodies,
        canopy: Body {
            position: f.position,
            velocity: f.velocity + f.up() * 12.,
            grounded: false,
        },
    };
    f.autopilot = false;
    f.assist = false;
    f.controls = DVec3::ZERO;
    f.status = "EJECTION / CANOPY JETTISON / REAR THEN FRONT".into();
    true
}
fn fall(b: &mut Body, dt: f64, drag: f64) {
    if b.grounded {
        return;
    }
    let up = b.position.normalize();
    let v = b.velocity.dot(up);
    let horizontal = b.velocity - up * v;
    b.velocity = horizontal * (-drag * dt).exp() + up * ((v - 9.81 * dt) * (-drag * dt).exp());
    b.position += b.velocity * dt;
    let ground = terrain_height(b.position.x, b.position.z).max(0.) + 0.4;
    if b.position.length() < RADIUS + ground {
        b.position = up * (RADIUS + ground);
        b.velocity = DVec3::ZERO;
        b.grounded = true;
    }
}
pub fn advance(f: &mut Flight, dt: f64) {
    if !f.ejection.active || !dt.is_finite() || dt <= 0. {
        return;
    }
    if dt > 1. / 240. {
        let n = (dt * 240.).ceil() as usize;
        for _ in 0..n {
            advance_step(f, dt / n as f64);
        }
    } else {
        advance_step(f, dt);
    }
}
fn advance_step(f: &mut Flight, dt: f64) {
    let old = f.ejection.age;
    f.ejection.age += dt;
    f.controls = DVec3::ZERO;
    f.autopilot = false;
    let q = f.ejection.attitude;
    fall(&mut f.ejection.canopy, dt, 0.14);
    for i in 0..2 {
        let t = f.ejection.age - delay(i);
        if t < 0. {
            f.ejection.pilots[i].position = f.position + f.orientation * seat_offset(i);
            f.ejection.pilots[i].velocity = f.velocity;
            f.ejection.seats[i] = f.ejection.pilots[i].clone();
            continue;
        }
        if old < delay(i) {
            f.ejection.pilots[i].velocity += q * DVec3::Y * 24.;
        }
        if t < 0.22 {
            f.ejection.pilots[i].velocity += q * DVec3::Y * 36. * dt;
        }
        let inflation = ((t - 2.) / 0.9).clamp(0., 1.);
        fall(&mut f.ejection.pilots[i], dt, 0.06 + 1.55 * inflation);
        if t < 1.35 {
            f.ejection.seats[i] = f.ejection.pilots[i].clone();
        } else {
            fall(&mut f.ejection.seats[i], dt, 0.10);
        }
    }
    f.status = if f.ejection.pilots[0].grounded {
        "AIRCREW DOWN / R TO RESET"
    } else if f.ejection.age > 3.5 {
        "AIRCREW UNDER PARACHUTES / R TO RESET"
    } else {
        "EJECTION IN PROGRESS / REAR THEN FRONT"
    }
    .into();
}
#[derive(Component)]
pub struct Escape(pub usize);
#[derive(Component)]
pub struct EscapeLabel;
#[derive(Component)]
pub struct EscapeSeat(usize);
#[derive(Component)]
pub struct Chute(usize, bool);
#[derive(Component)]
pub struct Canopy;
#[derive(Component)]
pub struct Rocket(usize);
pub fn setup(
    mut c: Commands,
    mut m: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    server: Res<AssetServer>,
) {
    use crate::cockpit::hornet_interior::{rod, seat};
    c.spawn((
        EscapeLabel,
        Text::new(""),
        TextFont {
            font_size: 19.,
            ..default()
        },
        TextColor(Color::srgb(1., 0.8, 0.25)),
        Node {
            position_type: PositionType::Absolute,
            left: percent(32.),
            top: percent(12.),
            ..default()
        },
    ));
    for i in 0..2 {
        let root = c
            .spawn((Escape(i), Transform::default(), Visibility::Hidden))
            .id();
        let sr = c
            .spawn((EscapeSeat(i), Transform::default(), Visibility::Hidden))
            .id();
        seat(&mut c, &mut m, &mut mats, sr);
        let glow = mats.add(StandardMaterial {
            base_color: Color::srgba(1., 0.55, 0.08, 0.7),
            emissive: LinearRgba::new(15., 4., 0.3, 1.),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            ..default()
        });
        let rocket = c
            .spawn((
                Rocket(i),
                Mesh3d(m.add(Cone::new(0.12, 1.5))),
                MeshMaterial3d(glow),
                Transform::default(),
                Visibility::Hidden,
                bevy::light::NotShadowCaster,
            ))
            .id();
        c.entity(sr).add_child(rocket);
        let chute = c
            .spawn((Chute(i, false), Transform::default(), Visibility::Hidden))
            .id();
        c.entity(root).add_child(chute);
        let cloth = mats.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.95,
            cull_mode: None,
            ..default()
        });
        let mut vertices = vec![];
        let mut colors = vec![];
        let mut indices = vec![];
        for j in 0..=12 {
            let a = j as f32 / 12. * std::f32::consts::FRAC_PI_2;
            for k in 0..=48 {
                let b = k as f32 / 48. * std::f32::consts::TAU;
                vertices.push([
                    3. * a.sin() * b.cos(),
                    4.4 + 1.6 * a.cos(),
                    3. * a.sin() * b.sin(),
                ]);
                colors.push(
                    [
                        [0.95, 0.38, 0.10, 1.],
                        [0.91, 0.88, 0.75, 1.],
                        [0.28, 0.34, 0.20, 1.],
                        [0.68, 0.61, 0.43, 1.],
                    ][k / 4 % 4],
                );
            }
        }
        for j in 0..12 {
            for k in 0..48 {
                let a = j * 49 + k;
                indices.extend([a, a + 49, a + 1, a + 1, a + 49, a + 50]);
            }
        }
        let mut mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh.insert_indices(bevy::mesh::Indices::U32(indices));
        mesh.compute_smooth_normals();
        let dome = m.add(mesh);
        let e = c
            .spawn((
                Mesh3d(dome.clone()),
                MeshMaterial3d(cloth.clone()),
                Transform::default(),
            ))
            .id();
        c.entity(chute).add_child(e);
        let drogue = c
            .spawn((Chute(i, true), Transform::default(), Visibility::Hidden))
            .id();
        c.entity(root).add_child(drogue);
        let e = c
            .spawn((Mesh3d(dome), MeshMaterial3d(cloth), Transform::default()))
            .id();
        c.entity(drogue).add_child(e);
        let cord = mats.add(StandardMaterial {
            base_color: Color::srgb(0.77, 0.76, 0.65),
            ..default()
        });
        for k in 0..24 {
            let a = k as f32 / 24. * std::f32::consts::TAU;
            for parent in [chute, drogue] {
                rod(
                    &mut c,
                    &mut m,
                    parent,
                    cord.clone(),
                    Vec3::new(a.cos() * 0.18, 0.65, a.sin() * 0.18),
                    Vec3::new(a.cos() * 3., 4.4, a.sin() * 3.),
                    0.007,
                );
            }
        }
    }
    let model = Transform::from_xyz(0., 0.35, 2.56)
        .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))
        .with_scale(Vec3::splat(0.1));
    let canopy = c
        .spawn((Canopy, Transform::default(), Visibility::Hidden))
        .id();
    let visual = c
        .spawn((
            SceneRoot(
                server
                    .load(GltfAssetLabel::Scene(0).from_asset("aircraft/fa18f/escape_canopy.glb")),
            ),
            model,
        ))
        .id();
    c.entity(canopy).add_child(visual);
}
pub fn visual(
    f: Res<Flight>,
    mut labels: Query<&mut Text, With<EscapeLabel>>,
    mut escape: Query<
        (&Escape, &mut Transform, &mut Visibility),
        (
            Without<EscapeSeat>,
            Without<Canopy>,
            Without<Chute>,
            Without<Rocket>,
        ),
    >,
    mut seats: Query<
        (&EscapeSeat, &mut Transform, &mut Visibility),
        (
            Without<Escape>,
            Without<Canopy>,
            Without<Chute>,
            Without<Rocket>,
        ),
    >,
    mut chutes: Query<
        (&Chute, &mut Transform, &mut Visibility),
        (
            Without<Escape>,
            Without<EscapeSeat>,
            Without<Canopy>,
            Without<Rocket>,
        ),
    >,
    mut canopy: Query<
        (&mut Transform, &mut Visibility),
        (
            With<Canopy>,
            Without<Escape>,
            Without<EscapeSeat>,
            Without<Chute>,
            Without<Rocket>,
        ),
    >,
    mut rockets: Query<
        (&Rocket, &mut Transform, &mut Visibility),
        (
            Without<Escape>,
            Without<EscapeSeat>,
            Without<Chute>,
            Without<Canopy>,
        ),
    >,
) {
    let active = f.ejection.active && f.aircraft == crate::aircraft::Aircraft::Hornet;
    for mut text in &mut labels {
        text.0 = if active {
            format!(
                "{} / {:.1}s / DESCENT {:.1} m/s",
                if f.ejection.age < 0.3 {
                    "CANOPY JETTISON"
                } else if f.ejection.age < 0.55 {
                    "REAR SEAT FIRING"
                } else if f.ejection.age < 1.9 {
                    "SEATS CLEAR / SEPARATION"
                } else if f.ejection.age < 3.5 {
                    "PARACHUTE DEPLOYMENT"
                } else {
                    "AIRCREW DESCENT / R RESET"
                },
                f.ejection.age,
                -f.ejection.pilots[0]
                    .velocity
                    .dot(f.ejection.pilots[0].position.normalize())
            )
        } else {
            String::new()
        };
    }
    for (tag, mut t, mut v) in &mut escape {
        let age = f.ejection.age - delay(tag.0);
        *v = if active && age >= 0. {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        t.translation = crate::world::local(f.ejection.pilots[tag.0].position, &f);
        t.rotation = crew_attitude(&f, tag.0);
    }
    for (tag, mut t, mut v) in &mut seats {
        let age = f.ejection.age - delay(tag.0);
        *v = if active && age >= 0. {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        t.translation = crate::world::local(f.ejection.seats[tag.0].position, &f);
        t.rotation = crate::world::local_rot(f.ejection.attitude, &f)
            * Quat::from_rotation_x(((age - 1.35).max(0.) * 0.8) as f32);
        let offset = t.rotation * Vec3::Y * 0.34;
        t.translation -= offset;
        t.scale = Vec3::new(0.75, 1., 1.);
    }
    for (tag, mut t, mut v) in &mut chutes {
        let age = f.ejection.age - delay(tag.0);
        let inflation = if tag.1 {
            if (0.85..2.9).contains(&age) { 0.18 } else { 0. }
        } else {
            ((age - 2.) / 0.9).clamp(0., 1.) as f32
        };
        *v = if active && inflation > 0. && !f.ejection.pilots[tag.0].grounded {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        t.scale = Vec3::splat(inflation.max(0.001));
        t.translation = if tag.1 { Vec3::Y * 1.5 } else { Vec3::ZERO };
    }
    for (mut t, mut v) in &mut canopy {
        *v = if active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        t.translation = crate::world::local(f.ejection.canopy.position, &f);
        t.rotation = crate::world::local_rot(f.ejection.attitude, &f)
            * Quat::from_rotation_x(f.ejection.age as f32 * 0.6);
    }
    for (tag, mut t, mut v) in &mut rockets {
        let age = f.ejection.age - delay(tag.0);
        *v = if active && (0.0..0.22).contains(&age) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        *t = Transform::from_xyz(0., -0.85, 0.2)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::PI));
    }
}
pub fn crew_attitude(f: &Flight, i: usize) -> Quat {
    let age = f.ejection.age - delay(i);
    let upright = DQuat::from_rotation_arc(DVec3::Y, f.ejection.pilots[i].position.normalize());
    crate::world::local_rot(
        f.ejection
            .attitude
            .slerp(upright, ((age - 1.35) / 1.0).clamp(0., 1.)),
        f,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sequential_launch_and_reset() {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Hornet);
        assert!(start(&mut f));
        assert!(!start(&mut f));
        for _ in 0..96 {
            advance(&mut f, 1. / 240.);
        }
        assert!((f.ejection.pilots[1].position - f.ejection.pilots[0].position).dot(f.up()) > 1.);
        for _ in 0..240 * 15 {
            advance(&mut f, 1. / 240.);
        }
        assert!(f.ejection.pilots.iter().all(|p| p.position.is_finite()));
        assert!(!Flight::approach().ejection.active);
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::F22);
        assert!(!f.ejection.active);
    }
}

#[cfg(test)]
mod escape_regressions {
    use super::*;
    #[test]
    fn near_level_stationary_escape_clears_the_ground_and_opens_chutes() {
        let mut f = Flight::runway();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Hornet);
        let ground = f.altitude();
        start(&mut f);
        for _ in 0..240 * 4 {
            advance(&mut f, 1. / 240.);
        }
        assert!(
            f.ejection
                .pilots
                .iter()
                .all(|p| !p.grounded && p.position.length() - RADIUS > ground + 3.)
        );
    }
    #[test]
    fn descent_after_inflation_is_bounded_and_repeatable() {
        let mut results = vec![];
        for hz in [60, 120, 240] {
            let mut f = Flight::approach();
            crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Hornet);
            f.velocity = DVec3::ZERO;
            start(&mut f);
            for _ in 0..hz * 12 {
                advance(&mut f, 1. / hz as f64);
            }
            let body = &f.ejection.pilots[0];
            let descent = body.velocity.dot(body.position.normalize());
            assert!(descent < -5. && descent > -8., "{descent}");
            results.push(body.position.length());
        }
        assert!((results[0] - results[2]).abs() < 1.5, "{results:?}");
    }
}
