//! Bounded Hornet impact presentation. Visual debris, not additional aircraft dynamics.
//! All motion uses simulation time; shared meshes/materials never change after startup.
use crate::{
    aircraft::Aircraft,
    carrier,
    physics::{Environment, Flight, RADIUS, Vehicle},
    world,
};
use bevy::{
    asset::RenderAssetUsages,
    math::{DQuat, DVec3},
    mesh::Indices,
    prelude::*,
    render::render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat},
};
const DEBRIS: usize = 24;
const COUNT: usize = 122;
#[derive(Component)]
pub struct Piece(usize);
#[derive(Clone, Copy, Default)]
struct Fragment {
    p: DVec3,
    v: DVec3,
    q: DQuat,
    spin: DVec3,
    settled: bool,
}
#[derive(Default, Resource)]
pub struct Scene {
    active: bool,
    last: f64,
    age: f64,
    origin: DVec3,
    up: DVec3,
    forward: DVec3,
    fuel: bool,
    water: bool,
    contact: Option<(DVec3, f64)>,
    fragments: [Fragment; DEBRIS],
}
// The airport's thin render slabs stand above its spherical collision terrain.
// Keep visual fragments above those surfaces, without changing aircraft contacts.
fn support(f: &Flight, p: DVec3) -> f64 {
    let base = carrier::ground(f, p);
    if carrier::deck(f, p).is_some() {
        return base;
    }
    let plane = if p.x.abs() < 22.5 && p.z.abs() < crate::physics::RUNWAY_HALF {
        Some(RADIUS + crate::physics::RUNWAY_HEIGHT + 0.23)
    } else if (p.x - 150.).abs() < 100. && (p.z - 620.).abs() < 325. {
        Some(
            crate::physics::surface_point(150., 620., crate::physics::RUNWAY_HEIGHT + 0.12).y
                + 0.05,
        )
    } else if (p.x - 80.).abs() < 85. && (p.z - 500.).abs() < 10. {
        Some(
            crate::physics::surface_point(80., 500., crate::physics::RUNWAY_HEIGHT + 0.15).y + 0.05,
        )
    } else if p.x.abs() < 90. && p.z.abs() < 1450. {
        Some(RADIUS + crate::physics::RUNWAY_HEIGHT)
    } else {
        None
    };
    plane.map_or(base, |y| {
        base.max(DVec3::new(p.x, y, p.z).length() - RADIUS)
    })
}
fn shape(i: usize) -> (DVec3, Vec3) {
    match i {
        0 => (DVec3::new(0., 0., -3.8), Vec3::new(1.35, 1.1, 5.3)),
        1 => (DVec3::new(0., 0., 1.8), Vec3::new(2.8, 0.85, 4.8)),
        2 | 3 => (
            DVec3::new(if i == 2 { -1.1 } else { 1.1 }, 0., 5.),
            Vec3::new(0.85, 2.8, 0.85),
        ),
        4 | 5 => (
            DVec3::new(if i == 4 { -4. } else { 4. }, 0., 1.5),
            Vec3::new(4.2, 0.12, 2.7),
        ),
        6 | 7 => (
            DVec3::new(if i == 6 { -2.2 } else { 2.2 }, 1.2, 5.2),
            Vec3::new(2., 0.1, 2.8),
        ),
        8 | 9 => (
            DVec3::new(if i == 8 { -2.5 } else { 2.5 }, 0., 6.),
            Vec3::new(2.4, 0.1, 1.8),
        ),
        10..=12 => (
            DVec3::new((i as f64 - 11.) * 1.8, -1., 0.),
            Vec3::new(0.65, 0.28, 0.65),
        ),
        _ => {
            let a = i as f64 * 2.39996;
            (
                DVec3::new(a.cos() * 2., -0.2, a.sin() * 4.),
                Vec3::new(
                    0.35 + (i % 3) as f32 * 0.4,
                    0.06,
                    0.6 + (i % 4) as f32 * 0.3,
                ),
            )
        }
    }
}
impl Scene {
    pub fn framing(&self) -> Option<(DVec3, f32)> {
        if !self.active || self.water {
            return None;
        }
        let center = (self.fragments[0].p + self.fragments[1].p) * 0.5;
        let radius = self
            .fragments
            .iter()
            .map(|p| p.p.distance(center))
            .fold(16., f64::max)
            .min(70.);
        Some((center, (radius * 2.3).max(38.) as f32))
    }

    fn start(&mut self, f: &Flight) {
        *self = Self {
            active: true,
            last: f.elapsed,
            origin: f.wreck_origin,
            up: f.wreck_origin.normalize(),
            fuel: f.fuel > 10.,
            water: f.water.active,
            ..default()
        };
        self.forward =
            (f.wreck_velocity - self.up * f.wreck_velocity.dot(self.up)).normalize_or_zero();
        if self.forward.length_squared() < 0.5 {
            self.forward = f.orientation * DVec3::NEG_Z;
        }
        let right = self.forward.cross(self.up).normalize_or_zero();
        let severity = (f.impact_energy / 1e6).sqrt().clamp(0.5, 8.);
        for (i, p) in self.fragments.iter_mut().enumerate() {
            let a = i as f64 * 2.39996;
            let offset = f.orientation * shape(i).0;
            let q = if matches!(i, 2 | 3) {
                f.orientation * DQuat::from_rotation_x(std::f64::consts::FRAC_PI_2)
            } else if matches!(i, 6 | 7) {
                f.orientation * DQuat::from_rotation_z(1.1)
            } else {
                f.orientation
            };
            *p = Fragment {
                p: self.origin + offset,
                v: f.wreck_velocity * (if i < 4 { 0.64 } else { 0.45 })
                    + right * a.cos() * severity * 1.7
                    + self.up * (2. + a.sin().abs() * severity)
                    + self.forward * a.sin() * severity,
                q,
                spin: DVec3::new(a.sin(), a.cos() * 0.7, 0.5) * if i < 4 { 0.7 } else { 2.5 },
                settled: false,
            };
        }
    }
    fn step(&mut self, f: &Flight, dt: f64) {
        if dt <= 0. {
            return;
        }
        // Contact is substepped independently from rendering; no long-frame tunnelling.
        let steps = (dt / (1. / 120.)).ceil().max(1.) as usize;
        let h = dt / steps as f64;
        for substep in 0..steps {
            let end_time=f.elapsed-dt+(substep+1)as f64*h;
            self.age += h;
            for (i, p) in self.fragments.iter_mut().enumerate() {
                let up = p.p.normalize();
                let size = shape(i).1.as_dvec3();
                let extent = 0.5
                    * ((p.q * DVec3::X).dot(up).abs() * size.x
                        + (p.q * DVec3::Y).dot(up).abs() * size.y
                        + (p.q * DVec3::Z).dot(up).abs() * size.z);
                let support = support(f, p.p) + extent.max(0.04);
                let gv = carrier::ground_velocity(f, p.p);
                p.v -= up * 9.81 * h;
                p.v *= (-h * 0.09).exp();
                let before=p.p;
                p.p += p.v * h;
                if let Some(hit)=carrier::collider::sweep_at(f.carrier.active,end_time,before,p.p,h,extent.max(0.04)) {
                    p.p=hit.point+hit.normal*0.02;
                    let gv=carrier::velocity(f.elapsed);let rel=p.v-gv;let vn=rel.dot(hit.normal);
                    p.v=gv+(rel-hit.normal*vn.min(0.)*1.17)*(-h*1.9).exp();
                    p.spin*=(-h*4.).exp();
                }
                if p.p.length() - RADIUS < support {
                    p.p = p.p.normalize() * (RADIUS + support);
                    if self.contact.is_none() && !self.water {
                        self.contact = Some((p.p, self.age));
                    }
                    let rel = p.v - gv;
                    let vn = rel.dot(up);
                    let tangent = rel - up * vn;
                    let bounced = if vn < -1.2 { -vn * 0.17 } else { 0. };
                    p.v = gv
                        + up * bounced
                        + tangent * (-h * if self.water { 2.5 } else { 1.9 }).exp();
                    p.spin *= (-h * 4.).exp();
                    if i >= 4 && tangent.length() < 8. {
                        let flat = DQuat::from_rotation_arc(p.q * DVec3::Y, up) * p.q;
                        p.q = p.q.slerp(flat, 1. - (-h * 2.).exp());
                    }
                    p.settled = tangent.length() < 0.3 && bounced == 0.;
                }
                if self.water {
                    p.v *= (-h * 0.7).exp();
                    if self.age > 4. {
                        p.p -= up * h * 0.35;
                    }
                }
                p.q = (DQuat::from_scaled_axis(p.spin * h) * p.q).normalize();
            }
        }
    }
}
// Extruded swept panels with torn roots and exposed thickness, rather than cubes.
fn panel_mesh(i: usize) -> Mesh {
    let outline: Vec<[f32; 2]> = if (4..=9).contains(&i) {
        vec![
            [-0.5, -0.48],
            [-0.18, -0.43],
            [0.5, 0.12],
            [0.42, 0.49],
            [-0.48, 0.37],
            [-0.39, 0.13],
            [-0.5, 0.04],
            [-0.37, -0.1],
        ]
    } else {
        vec![
            [-0.5, -0.45],
            [0.3, -0.5],
            [0.5, -0.22],
            [0.38, 0.5],
            [-0.35, 0.43],
            [-0.5, 0.16],
            [-0.38, 0.02],
        ]
    };
    let mut positions = vec![];
    let mut indices = vec![];
    for y in [-0.5, 0.5] {
        for [x, z] in &outline {
            positions.push([*x, y + 0.12 * (*x * 7.).sin(), *z]);
        }
    }
    let n = outline.len() as u32;
    // The torn outline is star-shaped around its interior centre. A fan from
    // a perimeter vertex crosses the notches and inverts some skin triangles.
    positions.extend([[0., -0.5, 0.], [0., 0.5, 0.]]);
    for k in 0..n {
        let next = (k + 1) % n;
        indices.extend([2 * n, k, next, 2 * n + 1, n + next, n + k]);
        indices.extend([k, n + next, next, k, n + k, n + next]);
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(Indices::U32(indices));
    mesh.duplicate_vertices();
    mesh.compute_flat_normals();
    mesh
}
pub fn setup(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    c.init_resource::<Scene>();
    let body = meshes.add(
        Capsule3d::new(0.5, 0.7)
            .mesh()
            .rings(8)
            .latitudes(12)
            .longitudes(20),
    );
    let engine = meshes.add(Cylinder::new(0.5, 1.).mesh().resolution(20));
    let wheel = meshes.add(Torus::new(0.22, 0.5));
    let cloud = meshes.add(Rectangle::new(1., 1.));
    // Soft, irregular radial extinction instead of opaque smoke balls.
    let mut pixels = vec![0u8; 64 * 64 * 4];
    for y in 0..64 {
        for x in 0..64 {
            let dx = (x as f32 - 31.5) / 31.5;
            let dy = (y as f32 - 31.5) / 31.5;
            let r = (dx * dx + dy * dy).sqrt();
            let noise = 0.78 + 0.22 * (dx * 11. + dy * 7.).sin() * (dy * 13. - dx * 5.).sin();
            let a = ((1. - r).max(0.).powf(1.4) * noise * 255.) as u8;
            let k = (y * 64 + x) * 4;
            pixels[k..k + 4].copy_from_slice(&[255, 255, 255, a]);
        }
    }
    let tex = images.add(Image::new(
        Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    let metal = mats.add(StandardMaterial {
        base_color: Color::srgb(0.19, 0.21, 0.22),
        metallic: 0.65,
        perceptual_roughness: 0.8,
        ..default()
    });
    let paint = mats.add(StandardMaterial {
        base_color: Color::srgb(0.39, 0.42, 0.44),
        metallic: 0.35,
        perceptual_roughness: 0.85,
        ..default()
    });
    let spar = meshes.add(Cuboid::default());
    let rubber = mats.add(StandardMaterial {
        base_color: Color::srgb(0.025, 0.025, 0.025),
        ..default()
    });
    let mut clouds = vec![];
    for (color, emission) in [
        (
            Color::srgba(1., 0.52, 0.08, 0.95),
            LinearRgba::new(70000., 24000., 2000., 1.),
        ),
        (Color::srgba(0.48, 0.40, 0.30, 0.40), LinearRgba::BLACK),
        (
            Color::srgba(1., 0.22, 0.015, 0.86),
            LinearRgba::new(1800., 600., 30., 1.),
        ),
        (Color::srgba(0.065, 0.067, 0.07, 0.75), LinearRgba::BLACK),
        (Color::srgba(0.035, 0.028, 0.022, 0.7), LinearRgba::BLACK),
    ] {
        clouds.push(mats.add(StandardMaterial {
            base_color: color,
            emissive: emission,
            base_color_texture: Some(tex.clone()),
            alpha_mode: AlphaMode::Blend,
            unlit: emission.red == 0.,
            cull_mode: None,
            depth_bias: 1.,
            ..default()
        }));
    }
    clouds.push(mats.add(StandardMaterial {
        base_color: Color::srgba(1., 0.85, 0.3, 0.9),
        emissive: LinearRgba::new(5000., 2800., 500., 1.),
        base_color_texture: Some(tex.clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: false,
        cull_mode: None,
        ..default()
    }));
    for i in 0..COUNT {
        let mesh = if i < DEBRIS {
            if matches!(i, 2 | 3) {
                engine.clone()
            } else if (10..=12).contains(&i) {
                wheel.clone()
            } else if i < 2 {
                body.clone()
            } else {
                meshes.add(panel_mesh(i))
            }
        } else {
            cloud.clone()
        };
        let mat = if i < DEBRIS {
            if (10..=12).contains(&i) {
                rubber.clone()
            } else if i >= 4 && i < 10 {
                paint.clone()
            } else {
                metal.clone()
            }
        } else {
            clouds[if i < 56 {
                0
            } else if i < 72 {
                1
            } else if i < 88 {
                if i % 4 < 2 { 5 } else { 2 }
            } else if i < 120 {
                3
            } else if i == 120 {
                4
            } else {
                0
            }]
            .clone()
        };
        let entity = c
            .spawn((
                Piece(i),
                Mesh3d(mesh),
                MeshMaterial3d(mat),
                Transform::default(),
                Visibility::Hidden,
                bevy::light::NotShadowCaster,
            ))
            .id();
        if i < DEBRIS {
            c.entity(entity).remove::<bevy::light::NotShadowCaster>();
        }
        // Exposed spars and ribs stay attached to each torn panel, including while tumbling.
        if (4..=9).contains(&i) {
            c.entity(entity).with_children(|parent| {
                for k in 0..4 {
                    parent.spawn((
                        Mesh3d(spar.clone()),
                        MeshMaterial3d(metal.clone()),
                        Transform::from_xyz(-0.34 + k as f32 * 0.20, -0.58, 0.)
                            .with_scale(Vec3::new(0.018, 0.45, 0.66 - k as f32 * 0.10)),
                        bevy::light::NotShadowCaster,
                    ));
                }
            });
        }
        if matches!(i, 2 | 3) {
            c.entity(entity).with_children(|parent| {
                for k in 0..3 {
                    parent.spawn((
                        Mesh3d(wheel.clone()),
                        MeshMaterial3d(metal.clone()),
                        Transform::from_xyz(0., -0.36 + k as f32 * 0.35, 0.)
                            .with_scale(Vec3::splat(0.93)),
                        bevy::light::NotShadowCaster,
                    ));
                }
            });
        }
    }
}
pub fn update(
    f: Res<Flight>,
    e: Res<Environment>,
    mut state: ResMut<Scene>,
    cameras: Query<&GlobalTransform, With<crate::FlightCamera>>,
    mut pieces: Query<(&Piece, &mut Transform, &mut Visibility)>,
) {
    let active = f.aircraft == Aircraft::Hornet && f.vehicle == Vehicle::Gripen && f.crashed;
    if !active {
        state.active = false;
        for (_, _, mut v) in &mut pieces {
            if *v != Visibility::Hidden {
                *v = Visibility::Hidden;
            }
        }
        return;
    }
    if !state.active || f.elapsed < state.last {
        state.start(&f);
        // Deterministic inspection poses use the same trajectory as the live effect.
        if f.paused && std::env::var("NORD_FLEET_CAPTURE").is_ok_and(|m| m.starts_with("crash-")) {
            let mut pose = f.clone();
            for _ in 0..(f.wreck_age * 120.).round() as usize {
                pose.elapsed += 1. / 120.;
                state.step(&pose, 1. / 120.);
            }
        }
    }
    // An airborne wreck may reach the sea after the breakup has started.
    state.water |= f.water.active;
    let dt = if f.paused {
        0.
    } else {
        (f.elapsed - state.last).clamp(0., 0.5)
    };
    state.last = f.elapsed;
    state.step(&f, dt);
    let age = state.age;
    let camera = cameras.iter().next();
    let wind = e.wind(state.origin);
    for (id, mut t, mut vis) in &mut pieces {
        let i = id.0;
        *vis = Visibility::Hidden;
        if i < DEBRIS {
            let p = state.fragments[i];
            t.translation = world::local(p.p, &f);
            t.rotation = world::local_rot(p.q, &f);
            t.scale = shape(i).1;
            if i < 2 {
                t.rotation *= Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
                t.scale = Vec3::new(shape(i).1.x, shape(i).1.z / 1.7, shape(i).1.y);
            }
            if !state.water || age < 12. {
                *vis = Visibility::Visible;
            }
            continue;
        }
        if state.water {
            continue;
        }
        let n = (i - DEBRIS) as f64;
        let phi = n * 2.39996;
        let side = state.forward.cross(state.up);
        let radial = side * phi.cos() + state.forward * phi.sin();
        let (p, scale, visible) = if i < 56 {
            let life = 0.35 + (i % 7) as f64 * 0.13;
            let v = state.forward * 18.
                + radial * (8. + (i % 9) as f64 * 2.)
                + state.up * (4. + (i % 5) as f64 * 2.);
            (
                state.origin + v * age - state.up * 4.905 * age * age,
                Vec3::new(0.08, 0.7, 1.) * (1. - age / life).max(0.) as f32,
                age < life,
            )
        } else if i < 72 {
            let (origin, time) = state.contact.unwrap_or((state.origin, age));
            let a = age - time - (i - 56) as f64 * 0.025;
            (
                origin
                    + radial * a.max(0.) * 3.
                    + state.forward * a.max(0.) * 3.
                    + state.up * (0.4 + a.max(0.) * 0.7)
                    + wind * a.max(0.) * 0.3,
                Vec3::splat(((2. + a.max(0.) * 5.) * (1. - a / 6.).clamp(0., 1.)) as f32),
                a > 0. && a < 6.,
            )
        } else if i < 88 {
            let source = state.fragments[if i % 2 == 0 { 0 } else { 1 }].p;
            let fade = (1. - age / 40.).clamp(0., 1.);
            let flicker = 0.75 + 0.25 * (age * 17. + phi).sin();
            (
                source + radial * 0.7 + state.up * (0.8 + (i % 4) as f64 * 0.4),
                Vec3::new(2.1, 3.8, 1.)
                    * (fade * flicker * if i % 4 < 2 { 0.55 } else { 1. }) as f32,
                state.fuel && age < 40.,
            )
        } else if i < 120 {
            // Recycle fixed puffs through the column, with gradual growth and dissipation.
            let delay = (i - 88) as f64 * 0.38;
            let a = (age - delay).rem_euclid(12.);
            let source = state.fragments[if i % 2 == 0 { 0 } else { 1 }];
            let size = (4. + a * 1.6) * (1. - a / 12.).sqrt();
            (
                source.p - source.v * a.min(1.)
                    + state.up * (3. + a * 3.1)
                    + wind * a
                    + radial * a * 0.22,
                Vec3::splat(size as f32),
                state.fuel && age > delay && age - a < 40. && age < 52.,
            )
        } else if i == 120 {
            let (origin, time) = state.contact.unwrap_or((state.origin, age));
            let shifted = origin + carrier::ground_velocity(&f, origin) * (age - time);
            let p = shifted.normalize() * (RADIUS + support(&f, shifted) + 0.035);
            (
                p,
                Vec3::new(13., 19., 1.),
                state.contact.is_some() && age - time > 0.15,
            )
        } else {
            let size = (age * 45.).min(12.) * (1. - age / 0.45).max(0.);
            (
                state.origin + state.up * 2.,
                Vec3::splat(size as f32),
                state.fuel && f.impact_energy > 1e6 && age < 0.45,
            )
        };
        if visible {
            t.translation = world::local(p, &f);
            t.scale = scale;
            t.rotation = if i == 120 {
                world::local_rot(DQuat::from_rotation_arc(DVec3::Z, state.up), &f)
            } else {
                camera.map_or(Quat::IDENTITY, |c| c.rotation())
            };
            *vis = Visibility::Visible;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn impact() -> Flight {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, Aircraft::Hornet);
        f.position = f.up() * (RADIUS + 3.);
        f.velocity = f.orientation * DVec3::new(0., -10., -90.);
        let up = f.up();
        crate::collision::destroy(&mut f, "TEST", up);
        f
    }
    #[test]
    fn debris_spreads_settles_and_stays_finite() {
        let f = impact();
        let mut s = Scene::default();
        s.start(&f);
        let p = s.fragments[4].p;
        for _ in 0..2400 {
            s.step(&f, 1. / 120.);
        }
        assert!(s.fragments[4].p.distance(p) > 5.);
        for p in s.fragments {
            assert!(p.p.is_finite() && p.v.is_finite());
            assert!(p.v.length() < 0.5);
            assert!((p.q.length() - 1.).abs() < 1e-10);
        }
    }
    #[test]
    fn pause_does_not_advance_debris() {
        let f = impact();
        let mut s = Scene::default();
        s.start(&f);
        let before = s.fragments[0].p;
        s.step(&f, 0.);
        assert_eq!(s.age, 0.);
        assert_eq!(s.fragments[0].p, before);
    }
    #[test]
    fn dry_tanks_and_water_are_recorded() {
        let mut f = impact();
        f.fuel = 0.;
        f.water.active = true;
        let mut s = Scene::default();
        s.start(&f);
        assert!(!s.fuel && s.water);
    }
    #[test]
    fn debris_contact_converges() {
        let f = impact();
        let mut a = Scene::default();
        a.start(&f);
        let mut b = Scene::default();
        b.start(&f);
        for _ in 0..600 {
            a.step(&f, 1. / 60.);
        }
        for _ in 0..1200 {
            b.step(&f, 1. / 120.);
        }
        for i in 0..DEBRIS {
            assert!(a.fragments[i].p.distance(b.fragments[i].p) < 0.01);
        }
    }
    #[test]
    fn render_sequence_cleans_up_on_reset_and_keeps_assets_bounded() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .insert_resource(impact())
            .insert_resource(Environment::default())
            .add_systems(Startup, setup)
            .add_systems(Update, update);
        app.update();
        let meshes = app.world().resource::<Assets<Mesh>>().len();
        let mats = app.world().resource::<Assets<StandardMaterial>>().len();
        for _ in 0..300 {
            app.world_mut().resource_mut::<Flight>().elapsed += 1. / 60.;
            app.update();
        }
        assert_eq!(meshes, app.world().resource::<Assets<Mesh>>().len());
        assert_eq!(
            mats,
            app.world().resource::<Assets<StandardMaterial>>().len()
        );
        {
            let w = app.world_mut();
            let count = w.query::<&Piece>().iter(w).count();
            assert_eq!(count, COUNT);
        }
        app.world_mut().resource_mut::<Flight>().crashed = false;
        app.update();
        {
            let w = app.world_mut();
            assert!(
                w.query_filtered::<&Visibility, With<Piece>>()
                    .iter(w)
                    .all(|v| *v == Visibility::Hidden)
            );
        }
        app.insert_resource(impact());
        app.update();
        {
            let w = app.world_mut();
            assert!(
                w.query::<(&Piece, &Visibility)>()
                    .iter(w)
                    .filter(|(p, _)| p.0 < DEBRIS)
                    .all(|(_, v)| *v == Visibility::Visible)
            );
        }
    }
    #[test]
    fn no_fire_or_smoke_over_water_or_without_fuel() {
        for water in [false, true] {
            let mut f = impact();
            f.water.active = false;
            if !water {
                f.fuel = 0.;
            }
            let mut app = App::new();
            app.add_plugins(MinimalPlugins)
                .init_resource::<Assets<Mesh>>()
                .init_resource::<Assets<StandardMaterial>>()
                .init_resource::<Assets<Image>>()
                .insert_resource(f)
                .insert_resource(Environment::default())
                .add_systems(Startup, setup)
                .add_systems(Update, update);
            app.update();
            if water {
                app.world_mut().resource_mut::<Flight>().water.active = true;
            }
            for _ in 0..120 {
                app.world_mut().resource_mut::<Flight>().elapsed += 1. / 60.;
                app.update();
            }
            let w = app.world_mut();
            assert!(
                w.query::<(&Piece, &Visibility)>()
                    .iter(w)
                    .filter(|(p, _)| (72..120).contains(&p.0) || p.0 == 121)
                    .all(|(_, v)| *v == Visibility::Hidden)
            );
        }
    }
    #[test]
    fn wreck_debris_does_not_fall_through_moving_deck() {
        let mut f=impact();f.carrier.active=true;f.elapsed=0.;
        f.position=carrier::world(DVec3::new(-10.,carrier::DECK+4.,0.),0.);
        f.wreck_origin=f.position;f.orientation=carrier::rotation(0.);
        f.wreck_velocity=carrier::velocity(0.)-f.up()*20.;
        let mut scene=Scene::default();scene.start(&f);
        for _ in 0..600 {f.elapsed+=1./120.;scene.step(&f,1./120.);}
        for p in scene.fragments {let q=carrier::relative(p.p,f.elapsed);
            if carrier::inside(q){assert!(q.y>=carrier::DECK-0.05,"debris below deck: {q:?}");}
        }
    }
    #[test]
    fn settled_skin_remains_above_the_visible_runway_slab() {
        use bevy::mesh::VertexAttributeValues;
        let mut f = Flight::runway();
        crate::aircraft::select(&mut f, Aircraft::Hornet);
        f.velocity = f.forward() * 50.;
        let up = f.up();
        crate::collision::destroy(&mut f, "TEST", up);
        let mut s = Scene::default();
        s.start(&f);
        for _ in 0..2400 {
            s.step(&f, 1. / 120.);
        }
        for i in 4..DEBRIS {
            let part = s.fragments[i];
            let mesh = panel_mesh(i);
            let VertexAttributeValues::Float32x3(points) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
            else {
                panic!("positions")
            };
            for p in points {
                let world = part.p + part.q * (Vec3::from_array(*p) * shape(i).1).as_dvec3();
                if world.x.abs() < 22.5 && world.z.abs() < crate::physics::RUNWAY_HALF {
                    assert!(
                        world.y >= RADIUS + crate::physics::RUNWAY_HEIGHT + 0.17,
                        "submerged panel {i}: {world:?}"
                    );
                }
            }
        }
    }
    #[test]
    fn settled_panels_have_outward_facing_skin() {
        use bevy::mesh::VertexAttributeValues;
        for i in 4..DEBRIS {
            let mesh = panel_mesh(i);
            let VertexAttributeValues::Float32x3(points) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
            else {
                panic!("positions")
            };
            let VertexAttributeValues::Float32x3(normals) =
                mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap()
            else {
                panic!("normals")
            };
            for (p, n) in points.iter().zip(normals) {
                if n[1].abs() > 0.6 {
                    assert!(p[1] * n[1] > 0., "panel {i}: inward skin {p:?} {n:?}");
                }
            }
        }
    }
    #[test]
    fn airborne_breakup_does_not_spawn_ground_dust_or_scorch() {
        let mut f = impact();
        f.position += f.up() * 3000.;
        f.wreck_origin = f.position;
        let mut scene = Scene::default();
        scene.start(&f);
        scene.step(&f, 0.5);
        assert!(scene.contact.is_none());
    }
}
