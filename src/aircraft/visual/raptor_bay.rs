//! Photo-guided side bay. The aperture, fitted door and rail share one definition.
//! Hidden structure and link lengths are visual approximations, not factory data.
use super::*;
use crate::aircraft::bays;

// Clockwise in the z/y plane; bevels avoid the old rectangular slab impression.
const SHIFT: f32 = -1.12;
const OUTLINE: [Vec2; 6] = [
    Vec2::new(-3.37, -0.14),
    Vec2::new(-0.20, -0.14),
    Vec2::new(-0.04, -0.32),
    Vec2::new(-0.16, -0.75),
    Vec2::new(-2.50, -0.75),
    Vec2::new(-3.37, -0.48),
];
const BACK: f32 = bays::BACK;
#[derive(Component)]
pub struct Door(pub usize);
#[derive(Component)]
pub struct Rail(pub usize);
#[derive(Component)]
struct MatingSeal;
#[derive(Component)]
struct HingeMount;
#[derive(Clone, Copy)]
enum End {
    Fixed(Vec3),
    Door(Vec3),
    Rail(Vec3),
    Elbow(f32),
}
#[derive(Component)]
pub struct Link {
    side: usize,
    a: End,
    b: End,
    radius: f32,
    start: f32,
    end: f32,
    joint: bool,
}

fn planes(side: f32) -> Vec<(Vec3, f32)> {
    let mut p = vec![(Vec3::X * side, BACK)];
    for i in 0..6 {
        let a = OUTLINE[i];
        let b = OUTLINE[(i + 1) % 6];
        let e = b - a;
        // Right-hand half-plane is the interior of this clockwise outline.
        let n = Vec3::new(0., -e.x, e.y).normalize();
        p.push((n, n.dot(Vec3::new(0., a.y, a.x))));
    }
    p
}
fn split(p: Vec<Vec3>, n: Vec3, d: f32) -> (Vec<Vec3>, Vec<Vec3>) {
    let mut inside = Vec::new();
    let mut outside = Vec::new();
    for i in 0..p.len() {
        let a = p[i];
        let b = p[(i + 1) % p.len()];
        let da = n.dot(a) - d;
        let db = n.dot(b) - d;
        if da >= 0. {
            inside.push(a)
        } else {
            outside.push(a)
        }
        if (da >= 0.) != (db >= 0.) {
            let q = a.lerp(b, da / (da - db));
            inside.push(q);
            outside.push(q);
        }
    }
    (inside, outside)
}
pub(super) fn cut(mut pieces: Vec<Vec<Vec3>>) -> Vec<Vec<Vec3>> {
    for side in [-1., 1.] {
        let planes = planes(side).into_iter().map(|(n,d)|(n,d+n.dot(bays::OFFSET))).collect::<Vec<_>>();
        pieces = pieces
            .into_iter()
            .flat_map(|mut p| {
                let mut outside = Vec::new();
                for &(n, d) in &planes {
                    if p.len() < 3 {
                        break;
                    }
                    let (a, b) = split(p, n, d);
                    p = a;
                    if b.len() >= 3 {
                        outside.push(b)
                    }
                }
                outside
            })
            .collect();
    }
    pieces
}
fn edge(side: f32, p: Vec2) -> Vec3 {
    side_point(Aircraft::F22, side, p.y+bays::OFFSET.y, p.x+bays::OFFSET.z)-bays::OFFSET
}
fn rail_pose(index:usize,extension:f64)->Transform {
    let mut t=bays::rail_pose(index,extension);t.translation-=bays::OFFSET;t
}
fn floor_at(z: f32) -> f32 {
    (0..6)
        .filter_map(|i| {
            let a = OUTLINE[i];
            let b = OUTLINE[(i + 1) % 6];
            if (b.x - a.x).abs() < 1e-6 || z < a.x.min(b.x) || z > a.x.max(b.x) {
                None
            } else {
                Some(a.y + (b.y - a.y) * (z - a.x) / (b.x - a.x))
            }
        })
        .fold(f32::INFINITY, f32::min)
}
fn inset_outline(inset: f32) -> [Vec2; 6] {
    let p = planes(1.);
    std::array::from_fn(|i| {
        let (a, da) = p[1 + (i + 5) % 6];
        let (b, db) = p[1 + i];
        let (a, b) = (Vec2::new(a.z, a.y), Vec2::new(b.z, b.y));
        let det = a.perp_dot(b);
        let da = da + inset;
        let db = db + inset;
        Vec2::new((da * b.y - db * a.y) / det, (a.x * db - b.x * da) / det)
    })
}
pub(super) fn hinge(side: f32) -> Vec3 {
    // The attachment depends only on the immutable authored F-22 skin.
    // Deriving it repeatedly for each moving link adds no new information.
    static HINGE_X: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    let x = *HINGE_X.get_or_init(|| (0..=64)
        .map(|i| {
            let z = -2.50 + 2.34 * i as f32 / 64.;
            edge(1., Vec2::new(z, -0.75)).x
        })
        .fold(f32::INFINITY, f32::min));
    Vec3::new(side * (x + 0.006), -0.75, -1.72)
}
pub(super) fn pose(index: usize, progress: f64) -> Transform {
    let s = bays::side(index);
    let p = progress as f32;
    let stand_off = bays::smooth(p / 0.22);
    let t = bays::smooth((p - 0.22) / 0.78);
    // A small supported stand-off clears the curved skin before the long hinge
    // rotates. The physical links below join these moving pivots to the frame.
    Transform::from_translation(hinge(s) + Vec3::X * s * 0.18 * stand_off)
        .with_rotation(Quat::from_rotation_z(-s * 1.75 * t))
}
pub(super) fn shell(side: f32) -> Mesh {
    let mut v = Vec::new();
    let mut ix = Vec::new();
    let h = hinge(side);
    let outline = inset_outline(0.006);
    for j in 0..44 {
        for k in 0..12 {
            let z = -3.37 + 3.33 * j as f32 / 44.;
            let y = -0.75 + 0.61 * k as f32 / 12.;
            let mut p = vec![
                Vec3::new(0., y, z),
                Vec3::new(0., y + 0.61 / 12., z),
                Vec3::new(0., y + 0.61 / 12., z + 3.33 / 44.),
                Vec3::new(0., y, z + 3.33 / 44.),
            ];
            for &(n, d) in &planes(side)[1..] {
                p = split(p, n, d + 0.006).0;
                if p.len() < 3 {
                    break;
                }
            }
            if p.len() < 3 {
                continue;
            }
            let p = p
                .into_iter()
                .map(|p| edge(side, Vec2::new(p.z, p.y)) + Vec3::X * side * 0.003 - h)
                .collect::<Vec<_>>();
            for i in 1..p.len() - 1 {
                let n = v.len() as u32;
                v.extend(if side > 0. {
                    [p[0], p[i], p[i + 1]]
                } else {
                    [p[0], p[i + 1], p[i]]
                });
                ix.extend([n, n + 1, n + 2]);
            }
            let p = p
                .iter()
                .map(|p| *p - Vec3::X * side * 0.018)
                .collect::<Vec<_>>();
            for i in 1..p.len() - 1 {
                let n = v.len() as u32;
                v.extend(if side > 0. {
                    [p[0], p[i + 1], p[i]]
                } else {
                    [p[0], p[i], p[i + 1]]
                });
                ix.extend([n, n + 1, n + 2]);
            }
        }
    }
    for i in 0..6 {
        for j in 0..32 {
            let a = edge(side, outline[i].lerp(outline[(i + 1) % 6], j as f32 / 32.))
                + Vec3::X * side * 0.003
                - h;
            let b = edge(
                side,
                outline[i].lerp(outline[(i + 1) % 6], (j + 1) as f32 / 32.),
            ) + Vec3::X * side * 0.003
                - h;
            quad(
                &mut v,
                &mut ix,
                [a, b, b - Vec3::X * side * 0.018, a - Vec3::X * side * 0.018],
            );
        }
    }
    let mut result = raptor_mesh::smooth(mesh(v, ix));
    // Paint in aircraft coordinates before attaching the moving door.
    result = result.transformed_by(Transform::from_translation(h));
    result = raptor_mesh::paint_at(result,bays::OFFSET);
    result.transformed_by(Transform::from_translation(-h))
}
fn inner_face(side: f32) -> Mesh {
    let shell = shell(side);
    let points = shell
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();
    let normals = shell
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .unwrap()
        .as_float3()
        .unwrap();
    let indices = shell.indices().unwrap().iter().collect::<Vec<_>>();
    let mut p = Vec::new();
    let mut ix = Vec::new();
    for t in indices.chunks_exact(3) {
        if t.iter().all(|i| normals[*i][0] * side < -0.3) {
            let n = p.len() as u32;
            p.extend(
                t.iter()
                    .map(|i| Vec3::from_array(points[*i]) - Vec3::X * side * 0.001),
            );
            ix.extend([n, n + 1, n + 2]);
        }
    }
    raptor_mesh::smooth(mesh(p, ix))
}
fn cavity(side: f32) -> Mesh {
    let mut v = Vec::new();
    let mut ix = Vec::new();
    let center = Vec3::new(side * BACK, -0.43, -1.72);
    for i in 0..6 {
        let a = OUTLINE[i];
        let b = OUTLINE[(i + 1) % 6];
        let n = v.len() as u32;
        v.extend([
            center,
            Vec3::new(side * BACK, a.y, a.x),
            Vec3::new(side * BACK, b.y, b.x),
        ]);
        ix.extend([n, n + 1, n + 2]);
        for j in 0..32 {
            let a = a.lerp(b, j as f32 / 32.);
            let b = OUTLINE[i].lerp(b, (j + 1) as f32 / 32.);
            quad(
                &mut v,
                &mut ix,
                [
                    edge(side, a),
                    edge(side, b),
                    Vec3::new(side * BACK, b.y, b.x),
                    Vec3::new(side * BACK, a.y, a.x),
                ],
            );
        }
    }
    raptor_mesh::smooth(mesh(v, ix))
}
pub(super) fn rigid(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    parent: Entity,
    shape: Mesh,
    mat: Handle<StandardMaterial>,
    t: Transform,
) -> Entity {
    let mut shape = shape;
    if !shape.contains_attribute(Mesh::ATTRIBUTE_COLOR) {
        shape.insert_attribute(
            Mesh::ATTRIBUTE_COLOR,
            vec![[1., 1., 1., 1.]; shape.count_vertices()],
        );
    }
    let e = part(c, m, parent, raptor_finish::uv(shape), mat, t);
    c.entity(e)
        .insert((crate::render_batch::RigidGeometry, Visibility::Inherited));
    e
}
pub(super) fn rod(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    parent: Entity,
    mat: Handle<StandardMaterial>,
    a: Vec3,
    b: Vec3,
    r: f32,
) -> Entity {
    rigid(
        c,
        m,
        parent,
        Cylinder::new(r, a.distance(b))
            .mesh()
            .resolution(12)
            .build(),
        mat,
        Transform::from_translation((a + b) * 0.5)
            .with_rotation(Quat::from_rotation_arc(Vec3::Y, (b - a).normalize())),
    )
}
pub(super) fn block(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    parent: Entity,
    mat: Handle<StandardMaterial>,
    at: Vec3,
    size: Vec3,
) {
    rigid(
        c,
        m,
        parent,
        Cuboid::from_size(size).into(),
        mat,
        Transform::from_translation(at),
    );
}
fn link(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    root: Entity,
    mat: Handle<StandardMaterial>,
    side: usize,
    a: End,
    b: End,
    r: f32,
    start: f32,
    end: f32,
) {
    let flat = r == 0.036 || r == 0.025;
    let shape = if flat {
        Cuboid::new(2., 1., 0.68).into()
    } else {
        Cylinder::new(1., 1.).mesh().resolution(16).build()
    };
    let e = part(c, m, root, shape, mat, Transform::default());
    c.entity(e).insert(Link {
        side,
        a,
        b,
        radius: r,
        start,
        end,
        joint: false,
    });
}
pub(super) fn setup(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    detail: &raptor_finish::Detail,
    root: Entity,
    skin: Handle<StandardMaterial>,
) {
    let parent=root;
    let root=c.spawn((Transform::from_translation(bays::OFFSET),Visibility::Inherited)).id();
    c.entity(parent).add_child(root);
    let skin = detail.apply(mats, &skin);
    let coated = material(mats, Color::srgb(0.73, 0.75, 0.70), 0.08);
    let coated = detail.apply(mats, &coated);
    let pale = material(mats, Color::srgb(0.84, 0.83, 0.74), 0.12);
    let metal = material(mats, Color::srgb(0.38, 0.40, 0.39), 0.78);
    let steel = material(mats, Color::srgb(0.65, 0.67, 0.66), 0.9);
    let seal = material(mats, Color::srgb(0.11, 0.13, 0.13), 0.03);
    for index in 0..2 {
        let s = bays::side(index);
        let h = hinge(s);
        rigid(c, m, root, cavity(s), coated.clone(), Transform::default());
        let door = c
            .spawn((Door(index), pose(index, 0.), Visibility::Inherited))
            .id();
        c.entity(root).add_child(door);
        rigid(c, m, door, shell(s), skin.clone(), Transform::default());
        rigid(
            c,
            m,
            door,
            inner_face(s),
            coated.clone(),
            Transform::default(),
        );
        // Continuous recessed gasket, inner frame and a restrained fastener row.
        let inner_outline = inset_outline(0.03);
        for i in 0..6 {
            let a = OUTLINE[i];
            let b = OUTLINE[(i + 1) % 6];
            let count = ((b - a).length() / 0.08).ceil() as usize;
            for j in 0..count {
                let p = a.lerp(b, j as f32 / count as f32);
                let q = a.lerp(b, (j + 1) as f32 / count as f32);
                let pa = edge(s, p) - Vec3::X * s * 0.025;
                let pb = edge(s, q) - Vec3::X * s * 0.025;
                let gasket = rod(c, m, root, seal.clone(), pa, pb, 0.008);
                c.entity(gasket).insert(MatingSeal);
                let p = edge(
                    s,
                    inner_outline[i].lerp(inner_outline[(i + 1) % 6], j as f32 / count as f32),
                ) - Vec3::X * s * 0.033
                    - h;
                let q = edge(
                    s,
                    inner_outline[i]
                        .lerp(inner_outline[(i + 1) % 6], (j + 1) as f32 / count as f32),
                ) - Vec3::X * s * 0.033
                    - h;
                rod(c, m, door, pale.clone(), p, q, 0.012);
                if j % 2 == 0 {
                    rigid(
                        c,
                        m,
                        door,
                        Cylinder::new(0.009, 0.004).mesh().resolution(8).build(),
                        steel.clone(),
                        Transform::from_translation(p - Vec3::X * s * 0.014)
                            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                    );
                }
            }
        }
        for z in [-1.96, -1.42, -0.88, -0.34, 0.20, 0.68] {
            let z = z + SHIFT;
            let floor = floor_at(z) + 0.05;
            // Formed ribs follow the actual curved door skin, rather than
            // floating above it as straight cuboids.
            for k in 0..12 {
                let a = edge(s, Vec2::new(z, floor + (-0.19 - floor) * k as f32 / 12.))
                    - h
                    - Vec3::X * s * 0.018;
                let b = edge(
                    s,
                    Vec2::new(z, floor + (-0.19 - floor) * (k + 1) as f32 / 12.),
                ) - h
                    - Vec3::X * s * 0.018;
                rod(c, m, door, coated.clone(), a, b, 0.005);
            }
            block(
                c,
                m,
                root,
                pale.clone(),
                Vec3::new(s * (BACK + 0.004), (floor - 0.20) * 0.5, z),
                Vec3::new(0.008, -0.20 - floor, 0.038),
            );
            // Latch pockets and catches on the lower sill.
            let p = edge(s, Vec2::new(z, floor));
            block(
                c,
                m,
                door,
                metal.clone(),
                p - h - Vec3::X * s * 0.04,
                Vec3::new(0.035, 0.055, 0.055),
            );
        }
        for z in [-1.88, -0.62, 0.62] {
            let z = z + SHIFT;
            // Bearings live on the recessed inner skin, not on a floating
            // straight bar outside the aircraft's curved closed-door contour.
            let y = floor_at(z) + if z < -2.6 {0.065} else {0.035};
            let skin = edge(s, Vec2::new(z, y)) - Vec3::X * s * 0.045;
            let pivot = skin;
            let support=rod(
                c,
                m,
                root,
                metal.clone(),
                Vec3::new(s * (BACK + 0.04), y, z),
                pivot - Vec3::X * s * 0.035,
                0.020,
            );
            c.entity(support).insert(HingeMount);
            rod(
                c,
                m,
                door,
                steel.clone(),
                pivot - h - Vec3::Z * 0.05,
                pivot - h + Vec3::Z * 0.05,
                0.025,
            );
            link(
                c,
                m,
                root,
                metal.clone(),
                index,
                End::Fixed(skin - Vec3::X * s * 0.035),
                End::Door(pivot - h),
                0.017,
                0.,
                1.,
            );
        }
        // Detailed launcher: two articulated parallel links, rigid rail,
        // missile suspension shoes, and retracting hydraulic cylinder/rod.
        let rail = c
            .spawn((
                Rail(index),
                rail_pose(index, 0.),
                Visibility::Inherited,
            ))
            .id();
        c.entity(root).add_child(rail);
        // Split channels leave the missile's upper fin envelope unobstructed.
        for x in [-0.05, 0.05] {
            block(
                c,
                m,
                rail,
                metal.clone(),
                Vec3::new(x, 0.115, 0.),
                Vec3::new(0.020, 0.04, 2.40),
            );
        }
        for x in [-0.052, 0.052] {
            block(
                c,
                m,
                rail,
                steel.clone(),
                Vec3::new(x, 0.073, 0.),
                Vec3::new(0.012, 0.018, 2.36),
            );
        }
        for z in [-0.40, 0.50] {
            block(
                c,
                m,
                rail,
                pale.clone(),
                Vec3::new(0., 0.13, z),
                Vec3::new(0.12, 0.06, 0.19),
            );
            let fixed = bays::launcher_pivot(index, z)-bays::OFFSET;
            block(
                c,
                m,
                root,
                pale.clone(),
                fixed - Vec3::X * s * 0.045 + Vec3::Y * 0.05,
                Vec3::new(0.055, 0.08, 0.18),
            );
            for dz in [-0.065, 0.065] {
                block(
                    c,
                    m,
                    root,
                    pale.clone(),
                    fixed + Vec3::Z * dz + Vec3::Y * 0.035,
                    Vec3::new(0.06, 0.07, 0.020),
                );
                rigid(
                    c,
                    m,
                    root,
                    Cylinder::new(0.022, 0.014).mesh().resolution(6).build(),
                    steel.clone(),
                    Transform::from_translation(fixed + Vec3::Z * dz * 1.25)
                        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                );
            }
            let moving = Vec3::new(0., 0.155, z);
            link(
                c,
                m,
                root,
                coated.clone(),
                index,
                End::Fixed(fixed),
                End::Elbow(z),
                0.036,
                0.,
                1.,
            );
            link(
                c,
                m,
                root,
                coated.clone(),
                index,
                End::Elbow(z),
                End::Rail(moving),
                0.036,
                0.,
                1.,
            );
            let pin = part(
                c,
                m,
                root,
                Cylinder::new(1., 0.75).mesh().resolution(16).build(),
                steel.clone(),
                Transform::default(),
            );
            c.entity(pin).insert(Link {
                side: index,
                a: End::Elbow(z),
                b: End::Elbow(z),
                radius: 0.032,
                start: 0.,
                end: 1.,
                joint: true,
            });
            rigid(
                c,
                m,
                rail,
                Cylinder::new(0.020, 0.22).mesh().resolution(16).build(),
                steel.clone(),
                Transform::from_translation(moving)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            );
            for dz in [-0.12, 0.12] {
                rigid(
                    c,
                    m,
                    rail,
                    Cylinder::new(0.024, 0.016).mesh().resolution(6).build(),
                    metal.clone(),
                    Transform::from_translation(moving + Vec3::Z * dz)
                        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                );
            }
        }
        let actuator_base=Vec3::new(s*(BACK+0.07),-0.57,-1.52+SHIFT);
        block(c,m,root,pale.clone(),actuator_base-Vec3::X*s*0.035,Vec3::new(0.07,0.12,0.16));
        let a = End::Fixed(actuator_base);
        let b = End::Rail(Vec3::new(0., 0.155, 0.42));
        link(c, m, root, metal.clone(), index, a, b, 0.042, 0., 0.62);
        link(c, m, root, steel.clone(), index, a, b, 0.021, 0.55, 1.);
        // Tubes remain clipped to the recessed back wall, with brackets and
        // junction fittings; no free-floating hoses across the missile path.
        for k in 0..3 {
            let x = s * (BACK + 0.006 + 0.002 * k as f32);
            let y = -0.46 - 0.045 * k as f32;
            rod(
                c,
                m,
                root,
                steel.clone(),
                Vec3::new(x, y, -2.02 + SHIFT),
                Vec3::new(x, y, 0.73 + SHIFT),
                0.003,
            );
            for z in [-1.80, -0.90, 0., 0.90] {
                let z = z + SHIFT;
                block(
                    c,
                    m,
                    root,
                    metal.clone(),
                    Vec3::new(x + s * 0.004, y, z),
                    Vec3::new(0.018, 0.026, 0.026),
                );
            }
        }
        for z in [-1.7, -0.4, 0.55] {
            block(
                c,
                m,
                root,
                pale.clone(),
                Vec3::new(s * (BACK + 0.018), -0.32, z + SHIFT),
                Vec3::new(0.025, 0.11, 0.22),
            );
        }
    }
}
pub fn update(
    f: Res<Flight>,
    mut previous: Local<Option<[bays::State; 2]>>,
    mut doors: Query<(&Door, &mut Transform), (Without<Rail>, Without<Link>)>,
    mut rails: Query<(&Rail, &mut Transform), (Without<Door>, Without<Link>)>,
    mut links: Query<(&Link, &mut Transform), (Without<Door>, Without<Rail>)>,
) {
    if f.aircraft != Aircraft::F22 {
        *previous = None;
        return;
    }
    if *previous == Some(f.systems.side_bays) { return; }
    *previous = Some(f.systems.side_bays);
    for (d, mut t) in &mut doors {
        t.set_if_neq(pose(d.0, f.systems.side_bays[d.0].door));
    }
    for (r, mut t) in &mut rails {
        t.set_if_neq(rail_pose(r.0, f.systems.side_bays[r.0].launcher));
    }
    for (l, mut t) in &mut links {
        let at = |e: End| match e {
            End::Fixed(p) => p,
            End::Door(p) => pose(l.side, f.systems.side_bays[l.side].door).transform_point(p),
            End::Rail(p) => {
                rail_pose(l.side, f.systems.side_bays[l.side].launcher).transform_point(p)
            }
            End::Elbow(z) => bays::launcher_joint(l.side, z, f.systems.side_bays[l.side].launcher)-bays::OFFSET,
        };
        let a = at(l.a);
        let b = at(l.b);
        let p = a.lerp(b, l.start);
        let q = a.lerp(b, l.end);
        if l.joint {
            t.set_if_neq(Transform::from_translation(a).with_scale(Vec3::splat(l.radius)));
            continue;
        }
        t.set_if_neq(
            Transform::from_translation((p + q) * 0.5)
                .with_rotation(Quat::from_rotation_arc(
                    Vec3::Y,
                    (q - p).normalize_or(Vec3::Y),
                ))
                .with_scale(Vec3::new(l.radius, p.distance(q), l.radius)),
        );
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    #[derive(Clone)]
    pub(crate) struct Triangle {
        pub(crate) p: [Vec3; 3],
        pub(crate) min: Vec3,
        pub(crate) max: Vec3,
        pub(crate) entity: Entity,
    }
    pub(crate) fn triangles(mesh: &Mesh, t: Mat4, entity: Entity) -> Vec<Triangle> {
        let p = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();
        let indices = mesh.indices().unwrap().iter().collect::<Vec<_>>();
        indices
            .chunks_exact(3)
            .filter_map(|i| {
                let p = [i[0], i[1], i[2]].map(|i| t.transform_point3(Vec3::from_array(p[i])));
                ((p[1] - p[0]).cross(p[2] - p[0]).length_squared() > 1e-14).then(|| Triangle {
                    min: p.into_iter().fold(Vec3::splat(f32::INFINITY), Vec3::min),
                    max: p
                        .into_iter()
                        .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max),
                    p,
                    entity,
                })
            })
            .collect()
    }
    pub(crate) fn intersects(a: &Triangle, b: &Triangle) -> bool {
        if (0..3).any(|i| a.min[i] > b.max[i] + 0.0001 || b.min[i] > a.max[i] + 0.0001) {
            return false;
        }
        // Mesh coordinates are f32, but narrow-phase projections need f64:
        // almost parallel skin/door faces otherwise lose their small clearance.
        let ap = a.p.map(|p| p.as_dvec3());
        let bp = b.p.map(|p| p.as_dvec3());
        let ae = [ap[1] - ap[0], ap[2] - ap[1], ap[0] - ap[2]];
        let be = [bp[1] - bp[0], bp[2] - bp[1], bp[0] - bp[2]];
        let an = ae[0].cross(ae[1]).normalize();
        let bn = be[0].cross(be[1]).normalize();
        // Parallel surfaces are mating/tangent contacts, not a penetrating
        // triangle pair. Nonparallel crossings use all separating axes.
        if an.dot(bn).abs() > 0.99999 {
            return false;
        }
        let separated = |n: bevy::math::DVec3| {
            if n.length_squared() < 1e-24 {
                return false;
            }
            let n = n.normalize();
            let range = |p: [bevy::math::DVec3; 3]| {
                p.map(|p| p.dot(n))
                    .into_iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| {
                        (lo.min(x), hi.max(x))
                    })
            };
            let (al, ah) = range(ap);
            let (bl, bh) = range(bp);
            // Touching is permitted; do not expand zero-thickness triangles
            // into a 50-micron solid and report a positive gap as penetration.
            ah <= bl + 1e-7 || bh <= al + 1e-7
        };
        !separated(an)
            && !separated(bn)
            && !ae
                .into_iter()
                .any(|a| be.into_iter().any(|b| separated(a.cross(b))))
    }
    #[test]
    fn narrow_phase_distinguishes_small_clearance_from_crossing() {
        let tri = |p: [[f32; 3]; 3]| {
            let p = p.map(Vec3::from_array);
            Triangle {
                min: p.into_iter().fold(Vec3::splat(f32::INFINITY), Vec3::min),
                max: p.into_iter().fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max),
                p,
                entity: Entity::PLACEHOLDER,
            }
        };
        let door = tri([
            [-1.70224, -0.57812995, -1.7902272],
            [-1.7496784, -0.53749996, -1.7145452],
            [-1.6961725, -0.5883333, -1.7145452],
        ]);
        let skin = tri([
            [-1.6867031, -0.5866846, -1.8907473],
            [-1.6876633, -0.59369373, -1.7603202],
            [-1.7250977, -0.54989654, -1.9014444],
        ]);
        assert!(!intersects(&door, &skin)); // 21 micrometres clear
        assert!(!intersects(&skin, &door));
        let a = tri([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]);
        let b = tri([[0.2, 0.2, -0.1], [0.6, 0.2, 0.1], [0.2, 0.6, 0.1]]);
        assert!(intersects(&a, &b));
        assert!(intersects(&b, &a));
        let tangent = tri([[0.2, 0.2, 0.], [0.6, 0.2, 0.1], [0.2, 0.6, 0.1]]);
        assert!(!intersects(&a, &tangent));
    }
    pub(crate) fn base_matrix(w: &World, e: Entity) -> Mat4 {
        let t = w
            .get::<Transform>(e)
            .map_or(Mat4::IDENTITY, Transform::to_matrix);
        w.get::<ChildOf>(e)
            .map_or(t, |p| base_matrix(w, p.parent()) * t)
    }
    fn dynamic(w: &World, mut e: Entity) -> bool {
        loop {
            if w.get::<Door>(e).is_some()
                || w.get::<Rail>(e).is_some()
                || w.get::<Link>(e).is_some()
                || w.get::<Store>(e).is_some()
            {
                return true;
            }
            let Some(p) = w.get::<ChildOf>(e) else {
                return false;
            };
            e = p.parent();
        }
    }
    pub(crate) fn belongs(w: &World, mut e: Entity, root: Entity) -> bool {
        loop {
            if e == root {
                return true;
            }
            let Some(p) = w.get::<ChildOf>(e) else {
                return false;
            };
            e = p.parent();
        }
    }
    fn moving_group(w: &World, mut e: Entity) -> Option<(usize, bool, Mat4)> {
        loop {
            if let Some(d) = w.get::<Door>(e) {
                return Some((d.0, false, base_matrix(w, e).inverse()));
            }
            if let Some(r) = w.get::<Rail>(e) {
                return Some((r.0, true, base_matrix(w, e).inverse()));
            }
            let Some(p) = w.get::<ChildOf>(e) else {
                return None;
            };
            e = p.parent();
        }
    }
    pub(crate) fn transformed(group: &[Triangle], t: Mat4) -> Vec<Triangle> {
        group
            .iter()
            .map(|a| {
                let p = a.p.map(|p| t.transform_point3(p));
                Triangle {
                    p,
                    entity: a.entity,
                    min: p.into_iter().fold(Vec3::splat(f32::INFINITY), Vec3::min),
                    max: p
                        .into_iter()
                        .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max),
                }
            })
            .collect()
    }
    #[test]
    fn assembled_airframe_and_loaded_bays_clear_every_one_percent_pose() {
        use std::collections::{HashMap, HashSet};
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .add_systems(Startup, super::super::setup);
        app.update();
        let w = app.world_mut();
        let root = w
            .query::<(Entity, &Root)>()
            .iter(w)
            .find(|(_, r)| r.0 == Aircraft::F22)
            .unwrap()
            .0;
        let mut fixed = Vec::new();let mut hinges=HashSet::new();
        let mut door_groups: [Vec<Triangle>; 2] = default();
        let mut rail_groups: [Vec<Triangle>; 2] = default();
        for (e, m, mat) in w
            .query::<(Entity, &Mesh3d, &MeshMaterial3d<StandardMaterial>)>()
            .iter(w)
        {
            if let Some((side, rail, inverse)) = moving_group(w, e) {
                let group = if rail {
                    &mut rail_groups[side]
                } else {
                    &mut door_groups[side]
                };
                group.extend(triangles(
                    w.resource::<Assets<Mesh>>().get(&m.0).unwrap(),
                    inverse * base_matrix(w, e),
                    e,
                ));
                continue;
            }
            if !belongs(w, e, root) || dynamic(w, e) || w.get::<MatingSeal>(e).is_some() {
                continue;
            }
            if w.get::<HingeMount>(e).is_some(){hinges.insert(e);}
            if w.resource::<Assets<StandardMaterial>>()
                .get(&mat.0)
                .unwrap()
                .alpha_mode
                != AlphaMode::Opaque
            {
                continue;
            }
            fixed.extend(
                triangles(
                    w.resource::<Assets<Mesh>>().get(&m.0).unwrap(),
                    base_matrix(w, e),
                    e,
                )
                .into_iter()
                .filter(|t| {
                    t.max.z >= -3.5
                        && t.min.z <= 1.10
                        && t.max.x >= -3.5
                        && t.min.x <= 3.5
                        && t.max.y >= -1.4
                        && t.min.y <= 0.1
                }),
            );
        }
        let cell = |p: Vec3| {
            (
                (p.x / 0.16).floor() as i32,
                (p.y / 0.16).floor() as i32,
                (p.z / 0.16).floor() as i32,
            )
        };
        let cells = |t: &Triangle| {
            let a = cell(t.min);
            let b = cell(t.max);
            let mut v = Vec::new();
            for x in a.0..=b.0 {
                for y in a.1..=b.1 {
                    for z in a.2..=b.2 {
                        v.push((x, y, z));
                    }
                }
            }
            v
        };
        let mut grid: HashMap<_, Vec<usize>> = HashMap::new();
        for (i, t) in fixed.iter().enumerate() {
            for key in cells(t) {
                grid.entry(key).or_default().push(i);
            }
        }
        let dummy = Entity::PLACEHOLDER;
        let missile = crate::combat::aim9x_mesh();
        let mut failures = Vec::new();
        let mut checks = 0usize;
        let mut seen = HashSet::new();
        for side in 0..2 {
            let s = bays::side(side);
            let hinge_position = hinge(s)+bays::OFFSET;
            for stage in (0..=200).step_by(25).chain((0..=200).filter(|s|s%25!=0)) {
                let state = if stage <= 100 {
                    bays::State {
                        door: stage as f64 / 100.,
                        launcher: 0.,
                    }
                } else {
                    bays::State {
                        door: 1.,
                        launcher: (stage - 100) as f64 / 100.,
                    }
                };
                let d = transformed(&door_groups[side], Mat4::from_translation(bays::OFFSET)*pose(side, state.door).to_matrix());
                let r = transformed(
                    &rail_groups[side],
                    bays::rail_pose(side, state.launcher).to_matrix(),
                );
                let missile = triangles(
                    &missile,
                    bays::missile_pose(side, state.launcher).to_matrix(),
                    dummy,
                );
                for (name, moving) in [("door", &d), ("rail", &r), ("missile", &missile)] {
                    for t in moving {
                        let center = t.p.into_iter().sum::<Vec3>() / 3.;
                        // Hinge bearings, the 18mm shell's perimeter seal and
                        // rail suspension shoes are intentional mating contacts.
                        if name == "door" && (center - hinge_position).truncate().length() < 0.10 {
                            continue;
                        }
                        let candidates = cells(t)
                            .into_iter()
                            .flat_map(|k| grid.get(&k).into_iter().flatten().copied())
                            .collect::<HashSet<_>>();
                        for i in candidates {
                            if name=="door" && hinges.contains(&fixed[i].entity){continue;}
                            checks += 1;
                            if intersects(t, &fixed[i])
                                && seen.insert((side, stage, name, fixed[i].entity))
                                && failures.len() < 100
                            {
                                failures.push(format!(
                                    "side {side} stage {stage} {name} at {center:?}: {:?} vs {:?} (vertices {}): {:?}",
                                    t.p, fixed[i].entity,
                                    w.resource::<Assets<Mesh>>().get(&w.get::<Mesh3d>(fixed[i].entity).unwrap().0).unwrap().count_vertices(),fixed[i].p
                                ));
                            }
                        }
                    }
                }
                let mut moving_grid: HashMap<_, Vec<usize>> = HashMap::new();
                for (i, t) in d.iter().enumerate() {
                    for key in cells(t) {
                        moving_grid.entry(key).or_default().push(i);
                    }
                }
                for (name, group) in [("missile/door", &missile), ("rail/door", &r)] {
                    for a in group {
                        let candidates = cells(a)
                            .into_iter()
                            .flat_map(|k| moving_grid.get(&k).into_iter().flatten().copied())
                            .collect::<HashSet<_>>();
                        for i in candidates {
                            if intersects(a, &d[i])
                                && seen.insert((side, stage, name, dummy))
                                && failures.len() < 100
                            {
                                failures.push(format!(
                                    "side {side} stage {stage}: {name} {:?} {:?}",
                                    a.p, d[i].p
                                ));
                            }
                        }
                    }
                }
                assert!(failures.is_empty(), "{}", failures.join("\n"));
            }
        }
        println!(
            "Full assembled bay sweep: {checks} triangle candidates, 402 opening/extension poses; closing traverses identical poses."
        );
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
    #[test]
    fn independent_mechanisms_and_shared_release_pose() {
        let mut a = bays::State::default();
        let b = a;
        a.advance(true, true, 1.);
        assert!(a.ready());
        assert_eq!(b, bays::State::default());
        for i in 0..2 {
            let mut f = Flight::runway();
            crate::aircraft::select(&mut f, Aircraft::F22);
            f.systems.side_bays[i] = a;
            let ammo = [2 - i as u8, 6];
            assert_eq!(
                crate::aircraft::launch_station(&f, crate::combat::Weapon::Heat, ammo),
                bays::missile_pose(i, 1.).translation.as_dvec3()
            );
        }
        for i in 0..2 {
            for z in [-0.40, 0.50] {
                for step in 0..=100 {
                    let pose = bays::rail_pose(i, step as f64 / 100.);
                    let elbow = bays::launcher_joint(i, z, step as f64 / 100.);
                    for length in [
                        elbow.distance(bays::launcher_pivot(i, z)),
                        elbow.distance(pose.transform_point(Vec3::new(0., 0.155, z))),
                    ] {
                        assert!((length - 0.72).abs() < 1e-5, "link stretched: {length}");
                    }
                }
            }
        }
    }
}
