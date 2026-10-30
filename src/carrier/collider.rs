//! Ship collision skin exported from the visible GLB, queried in the moving ship frame.
//! BVH broad phase + continuous convex/triangle SAT and swept sphere contacts.
use super::*;
use std::sync::OnceLock;
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub fraction: f64,
    pub normal: DVec3,
    pub point: DVec3,
    pub ship: usize,
}
#[derive(Clone, Copy)]
struct Triangle {
    p: [DVec3; 3],
    lo: DVec3,
    hi: DVec3,
}
struct Node {
    lo: DVec3,
    hi: DVec3,
    start: usize,
    end: usize,
    children: Option<(usize, usize)>,
}
struct Skin {
    tri: Vec<Triangle>,
    nodes: Vec<Node>,
}
fn overlaps(a: DVec3, b: DVec3, c: DVec3, d: DVec3) -> bool {
    !b.cmplt(c).any() && !a.cmpgt(d).any()
}
impl Skin {
    fn load(data: &[u8]) -> Self {
        assert_eq!(data.len() % 36, 0);
        let mut tri = Vec::with_capacity(data.len() / 36);
        for bytes in data.chunks_exact(36) {
            let mut p = [DVec3::ZERO; 3];
            for (i, v) in bytes.chunks_exact(4).enumerate() {
                p[i / 3][i % 3] = f32::from_le_bytes(v.try_into().unwrap()) as f64;
            }
            if (p[1] - p[0]).cross(p[2] - p[0]).length_squared() < 1e-16 {
                continue;
            }
            tri.push(Triangle {
                p,
                lo: p[0].min(p[1]).min(p[2]),
                hi: p[0].max(p[1]).max(p[2]),
            });
        }
        let mut skin = Self { tri, nodes: vec![] };
        let end = skin.tri.len();
        skin.build(0, end);
        skin
    }
    fn build(&mut self, start: usize, end: usize) -> usize {
        let lo = self.tri[start..end]
            .iter()
            .fold(DVec3::splat(f64::INFINITY), |a, t| a.min(t.lo));
        let hi = self.tri[start..end]
            .iter()
            .fold(DVec3::splat(f64::NEG_INFINITY), |a, t| a.max(t.hi));
        let index = self.nodes.len();
        self.nodes.push(Node {
            lo,
            hi,
            start,
            end,
            children: None,
        });
        if end - start > 8 {
            let d = hi - lo;
            let axis = if d.x > d.y && d.x > d.z {
                0
            } else if d.y > d.z {
                1
            } else {
                2
            };
            self.tri[start..end].sort_unstable_by(|a, b| {
                (a.lo[axis] + a.hi[axis]).total_cmp(&(b.lo[axis] + b.hi[axis]))
            });
            let mid = (start + end) / 2;
            let a = self.build(start, mid);
            let b = self.build(mid, end);
            self.nodes[index].children = Some((a, b));
        }
        index
    }
    fn query(&self, lo: DVec3, hi: DVec3, mut visit: impl FnMut(&Triangle)) {
        let mut stack = [0usize; 64];
        let mut count = 1;
        while count > 0 {
            count -= 1;
            let node = &self.nodes[stack[count]];
            if !overlaps(lo, hi, node.lo, node.hi) {
                continue;
            }
            if let Some((a, b)) = node.children {
                stack[count] = a;
                stack[count + 1] = b;
                count += 2;
            } else {
                for t in &self.tri[node.start..node.end] {
                    if overlaps(lo, hi, t.lo, t.hi) {
                        visit(t);
                    }
                }
            }
        }
    }
}
pub fn preload() {
    let _ = skin();

    let _ = burke_skin();
}
fn skin() -> &'static Skin {
    static SKIN: OnceLock<Skin> = OnceLock::new();
    SKIN.get_or_init(|| Skin::load(include_bytes!("../../assets/ships/nimitz/collision.bin")))
}
fn burke_skin() -> &'static Skin {
    static SKIN: OnceLock<Skin> = OnceLock::new();
    SKIN.get_or_init(|| Skin::load(include_bytes!("../../assets/ships/burke/collision.bin")))
}
fn ship_skin(ship: usize) -> &'static Skin {
    if ship == 0 { skin() } else { burke_skin() }
}
fn ship_offset(ship: usize) -> DVec3 {
    if ship == 0 {
        DVec3::ZERO
    } else {
        super::escorts::offset(ship - 1)
    }
}

fn near_ship(a: DVec3, b: DVec3, pad: f64, ship: usize) -> bool {
    if ship == 0 {
        return near(a, b, pad);
    }
    overlaps(
        a.min(b) - DVec3::splat(pad),
        a.max(b) + DVec3::splat(pad),
        DVec3::new(-14., -11., -80.),
        DVec3::new(14., 50., 80.),
    )
}
fn near(a: DVec3, b: DVec3, pad: f64) -> bool {
    overlaps(
        a.min(b) - DVec3::splat(pad),
        a.max(b) + DVec3::splat(pad),
        DVec3::new(-45., -10., -170.),
        DVec3::new(45., 70., 170.),
    )
}
fn result(end_time: f64, local: (f64, DVec3), a: DVec3, b: DVec3, dt: f64, ship: usize) -> Hit {
    let time = end_time - dt + dt * local.0;
    Hit {
        fraction: local.0,
        normal: rotation(time) * local.1,
        point: a.lerp(b, local.0),
        ship,
    }
}
/// Earliest contact of the complete, articulated aircraft skin. Rotation is
/// bounded in short angular intervals, including the ship's change of frame.
pub fn aircraft(f: &Flight, previous: DVec3, old_q: DQuat, dt: f64) -> Option<Hit> {
    if !f.carrier.active {
        return None;
    }

    (0..3)
        .filter_map(|ship| aircraft_ship(f, previous, old_q, dt, ship))
        .min_by(|a, b| a.fraction.total_cmp(&b.fraction))
}
fn aircraft_ship(f: &Flight, previous: DVec3, old_q: DQuat, dt: f64, ship: usize) -> Option<Hit> {
    let start_time = f.elapsed - dt;
    let a = relative(previous, start_time) - ship_offset(ship);
    let b = relative(f.position, f.elapsed) - ship_offset(ship);
    let radius = f.aircraft.spec().length.max(20.);
    if !near_ship(a, b, radius, ship) {
        return None;
    }
    let qa = rotation(start_time).conjugate() * old_q;
    let qb = rotation(f.elapsed).conjugate() * f.orientation;
    let angle = qa.angle_between(qb);
    let steps = (angle / 0.025).ceil().clamp(1., 128.) as usize;
    let margin = radius * (angle / steps as f64 * 0.5).sin().abs() + 0.012;
    let mut first: Option<(f64, DVec3)> = None;
    for hull in crate::airframe::for_flight(f) {
        if !hull.enabled(f) {
            continue;
        }
        let (hinge, offset) = hull.pose(f);
        let (min, max) = hull.bounds();
        let half = (max - min) * 0.5;
        let center = (max + min) * 0.5;
        for step in 0..steps {
            let t0 = step as f64 / steps as f64;
            let t1 = (step + 1) as f64 / steps as f64;
            let q = qa.slerp(qb, (t0 + t1) * 0.5);
            let r = q * hinge;
            let origin = a.lerp(b, t0) + q * offset;
            let delta = (b - a) * (t1 - t0);
            let extent = (r * (DVec3::X * half.x)).abs()
                + (r * (DVec3::Y * half.y)).abs()
                + (r * (DVec3::Z * half.z)).abs()
                + DVec3::splat(margin);
            let c = origin + r * center;
            ship_skin(ship).query(
                c.min(c + delta) - extent,
                c.max(c + delta) + extent,
                |tri| {
                    if let Some((t, n)) = hull.sweep_triangle(origin, delta, r, tri.p, margin) {
                        let time = t0 + (t1 - t0) * t;
                        if first.is_none_or(|h| time < h.0) {
                            first = Some((time, n));
                        }
                    }
                },
            );
        }
    }
    // Gear may brush the deck as part of its suspension, but tires and struts
    // still hit vertical hull/island faces and structures above the landing plane.
    if f.gear_extension > 0.001 {
        for i in 0..3 {
            if f.gear_health[i] <= 0. {
                continue;
            }
            let (mount, mut wheel, _) = crate::collision::gear_pose(f, i, true);
            let tire_radius = if f.aircraft == Aircraft::Hornet && f.gear_extension > 0.98 {
                let (p, r) = crate::aircraft::hornet::wheel(i);
                wheel = p + DVec3::Y * f.gear_compression[i];
                r
            } else {
                crate::collision::WHEEL_RADIUS
            };
            for (point, radius) in [(wheel, tire_radius), ((mount + wheel) * 0.5, 0.12)] {
                let ga = a + qa * point;
                let gb = b + qb * point;
                ship_skin(ship).query(
                    ga.min(gb) - DVec3::splat(radius),
                    ga.max(gb) + DVec3::splat(radius),
                    |tri| {
                        let normal = (tri.p[1] - tri.p[0]).cross(tri.p[2] - tri.p[0]).normalize();
                        if ship == 0 && tri.hi.y < DECK + 0.6 {
                            // Suspension owns the top face. A wheel rolling off the bow must also
                            // be able to leave a side face from the inside of the supporting deck.
                            if normal.y.abs() > 0.6 || (gb - ga).dot(normal) > 1e-9 {
                                return;
                            }
                        }
                        if let Some((t, n)) = sphere_triangle(ga, gb, radius, tri.p) {
                            if first.is_none_or(|h| t < h.0) {
                                first = Some((t, n));
                            }
                        }
                    },
                );
            }
        }
    }

    first.map(|h| result(f.elapsed, h, previous, f.position, dt, ship))
}
/// Swept world-space sphere used by debris, projectiles and wreck transport.
pub fn sweep(f: &Flight, a: DVec3, b: DVec3, dt: f64, radius: f64) -> Option<Hit> {
    sweep_at(f.carrier.active, f.elapsed, a, b, dt, radius)
}
pub fn sweep_at(
    active: bool,
    end_time: f64,
    a: DVec3,
    b: DVec3,
    dt: f64,
    radius: f64,
) -> Option<Hit> {
    if !active {
        return None;
    }

    (0..3)
        .filter_map(|ship| sweep_ship(end_time, a, b, dt, radius, ship))
        .min_by(|a, b| a.fraction.total_cmp(&b.fraction))
}
fn sweep_ship(end_time: f64, a: DVec3, b: DVec3, dt: f64, radius: f64, ship: usize) -> Option<Hit> {
    let start = relative(a, end_time - dt) - ship_offset(ship);
    let end = relative(b, end_time) - ship_offset(ship);
    if !near_ship(start, end, radius, ship) {
        return None;
    }
    let mut first: Option<(f64, DVec3)> = None;
    ship_skin(ship).query(
        start.min(end) - DVec3::splat(radius),
        start.max(end) + DVec3::splat(radius),
        |tri| {
            if let Some(h) = sphere_triangle(start, end, radius, tri.p) {
                if first.is_none_or(|p| h.0 < p.0) {
                    first = Some(h);
                }
            }
        },
    );
    first.map(|h| result(end_time, h, a, b, dt, ship))
}
fn sphere_triangle(a: DVec3, b: DVec3, r: f64, p: [DVec3; 3]) -> Option<(f64, DVec3)> {
    let delta = b - a;
    let normal = (p[1] - p[0]).cross(p[2] - p[0]).normalize();
    let mut best: Option<(f64, DVec3)> = None;
    let mut hit = |t: f64, n: DVec3| {
        if (0. ..=1.).contains(&t) && best.is_none_or(|v| t < v.0) {
            best = Some((t, n.normalize_or_zero()));
        }
    };
    let distance = (a - p[0]).dot(normal);
    let speed = delta.dot(normal);
    let inside =
        |q: DVec3| (0..3).all(|i| (p[(i + 1) % 3] - p[i]).cross(q - p[i]).dot(normal) >= -1e-8);
    let projected = a - normal * distance;
    if distance.abs() <= r + 1e-8 && inside(projected) {
        hit(0., if distance >= 0. { normal } else { -normal });
    }
    if speed.abs() > 1e-14 {
        for sign in [-1., 1.] {
            let t = (sign * r - distance) / speed;
            let q = a + delta * t - normal * (sign * r);
            if inside(q) {
                hit(t, normal * sign);
            }
        }
    }
    // Edge capsules + vertex spheres close the swept sphere at triangle boundaries.
    for i in 0..3 {
        let e = p[(i + 1) % 3] - p[i];
        let m = a - p[i];
        let e2 = e.length_squared();
        let u = m.dot(e) / e2;
        let v = delta.dot(e) / e2;
        let side = m - e * u;
        let dir = delta - e * v;
        if let Some(t) = root(
            dir.length_squared(),
            2. * side.dot(dir),
            side.length_squared() - r * r,
        ) {
            if (0. ..=1.).contains(&(u + v * t)) {
                hit(t, side + dir * t);
            }
        }
        if let Some(t) = root(
            delta.length_squared(),
            2. * m.dot(delta),
            m.length_squared() - r * r,
        ) {
            hit(t, m + delta * t);
        }
    }
    best
}
fn root(a: f64, b: f64, c: f64) -> Option<f64> {
    if c <= 0. {
        return Some(0.);
    }
    if a < 1e-20 {
        return None;
    }
    let d = b * b - 4. * a * c;
    if d < 0. {
        return None;
    }
    let t = (-b - d.sqrt()) / (2. * a);
    (t >= 0. && t <= 1.).then_some(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn flight(time: f64) -> Flight {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, Aircraft::Hornet);
        f.carrier.active = true;
        f.elapsed = time;
        f.orientation = rotation(time);
        f.gear = false;
        f.gear_extension = 0.;
        f
    }
    #[test]
    fn sphere_hits_face_edge_and_vertex_without_tunnelling() {
        let t = [
            DVec3::new(-1., 0., -1.),
            DVec3::new(1., 0., -1.),
            DVec3::new(0., 0., 1.),
        ];
        for (a, b, r) in [
            (DVec3::Y * 100., DVec3::NEG_Y * 100., 0.),
            (DVec3::new(0., 1., -1.1), DVec3::new(0., -1., -1.1), 0.2),
            (DVec3::new(0., 1., 1.1), DVec3::new(0., -1., 1.1), 0.2),
        ] {
            let hit = sphere_triangle(a, b, r, t).unwrap();
            assert!(hit.0 > 0. && hit.0 < 0.51);
            assert!(hit.1.dot(b - a) < 0.);
        }
        assert!(sphere_triangle(DVec3::new(3., 1., 0.), DVec3::new(3., -1., 0.), 0.1, t).is_none());
    }
    #[test]
    fn actual_deck_sides_underside_and_island_are_solid() {
        let f = flight(10.);
        for (name, a, b) in [
            (
                "deck",
                DVec3::new(-10., DECK + 10., 0.),
                DVec3::new(-10., DECK - 20., 0.),
            ),
            ("side", DVec3::new(60., 10., 0.), DVec3::new(0., 10., 0.)),
            (
                "underside",
                DVec3::new(0., -20., 0.),
                DVec3::new(0., 10., 0.),
            ),
            (
                "stern",
                DVec3::new(0., 10., 190.),
                DVec3::new(0., 10., 100.),
            ),
            (
                "bow",
                DVec3::new(0., 10., -190.),
                DVec3::new(0., 10., -100.),
            ),
            (
                "island",
                DVec3::new(-10., 40., 60.),
                DVec3::new(45., 40., 60.),
            ),
        ] {
            let h = sweep(&f, world(a, 9.9), world(b, 10.), 0.1, 0.1)
                .unwrap_or_else(|| panic!("missed {name}"));
            assert!(h.fraction > 0. && h.fraction < 1., "{name} {h:?}");
            assert!(h.normal.dot(world(b, 10.) - world(a, 9.9)) < 0.);
        }
        assert!(
            sweep(
                &f,
                world(DVec3::new(60., 70., 0.), 9.9),
                world(DVec3::new(60., -5., 0.), 10.),
                0.1,
                0.1
            )
            .is_none()
        );
    }
    #[test]
    fn swept_airframe_catches_fast_deck_and_island_crossings() {
        for (a, b) in [
            (
                DVec3::new(-10., DECK + 12., 0.),
                DVec3::new(-10., DECK - 20., 0.),
            ),
            (DVec3::new(-10., 40., 60.), DVec3::new(45., 40., 60.)),
        ] {
            let mut f = flight(5.);
            f.position = world(b, 5.);
            f.velocity = rotation(5.) * (b - a) * 10.;
            crate::collision::check_airframe_dt(&mut f, world(a, 4.9), rotation(4.9), 0.1);
            assert!(f.crashed && f.status.contains("CARRIER COLLISION"));
            assert!(relative(f.position, 5.).distance(b) > 1.);
        }
    }
    #[test]
    fn a_wing_hits_the_island_while_the_fuselage_path_is_clear() {
        let mut f = flight(2.);
        let a = DVec3::new(5., 40., 60.);
        let b = DVec3::new(23., 40., 60.);
        let before = world(a, 1.9);
        f.position = world(b, 2.);
        assert!(sweep(&f, before, f.position, 0.1, 0.1).is_none());
        assert!(aircraft(&f, before, rotation(1.9), 0.1).is_some());
    }
    #[test]
    fn moving_frame_preserves_contact_time_and_paused_ship_is_static() {
        let a = DVec3::new(-10., DECK + 12., 0.);
        let b = DVec3::new(-10., DECK - 12., 0.);
        let mut reference: Option<f64> = None;
        for time in [0.1, 120., 3600.] {
            let f = flight(time);
            let h = sweep(&f, world(a, time - 0.1), world(b, time), 0.1, 0.3).unwrap();
            if let Some(t) = reference {
                assert!((h.fraction - t).abs() < 1e-7);
            }
            reference = Some(h.fraction);
            assert!(sweep(&f, world(a, time), world(a, time), 0., 0.3).is_none());
        }
    }
    #[test]
    fn bvh_agrees_with_bruteforce_mesh_for_deterministic_rays() {
        for i in 0..24 {
            let a = DVec3::new(-50. + i as f64 * 4., 70., -155. + i as f64 * 13.);
            let b = DVec3::new(a.x + 9., -15., a.z - 5.);
            let mut accelerated: Option<f64> = None;
            skin().query(a.min(b), a.max(b), |t| {
                if let Some(h) = sphere_triangle(a, b, 0., t.p) {
                    accelerated = Some(accelerated.map_or(h.0, |x| x.min(h.0)));
                }
            });
            let brute = skin()
                .tri
                .iter()
                .filter_map(|t| sphere_triangle(a, b, 0., t.p).map(|v| v.0))
                .reduce(f64::min);
            assert_eq!(accelerated.is_some(), brute.is_some());
            if let (Some(a), Some(b)) = (accelerated, brute) {
                assert!((a - b).abs() < 1e-10);
            }
        }
    }
    #[test]
    fn both_escort_hulls_intercept_objects_and_gaps_remain_empty() {
        let f = flight(60.);
        for (ship, x) in [(1, -500.), (2, 500.)] {
            let a = world(DVec3::new(x + 30., 4., 0.), 59.9);
            let b = world(DVec3::new(x - 30., 4., 0.), 60.);
            let hit = sweep(&f, a, b, 0.1, 0.2).expect("escort hull contact");
            assert_eq!(hit.ship, ship);
            assert!(hit.fraction > 0. && hit.fraction < 1.);
        }
        assert!(
            sweep(
                &f,
                world(DVec3::new(250., 50., 0.), 59.9),
                world(DVec3::new(250., -5., 0.), 60.),
                0.1,
                0.2
            )
            .is_none()
        );
    }
    #[test]
    fn fleet_queries_choose_the_first_ship_along_the_trajectory() {
        let f = flight(1.);
        let hit = sweep(
            &f,
            world(DVec3::new(-600., 4., 0.), 0.9),
            world(DVec3::new(600., 4., 0.), 1.),
            0.1,
            0.1,
        )
        .unwrap();
        assert_eq!(hit.ship, 1);
        assert!(hit.fraction < 0.2);
    }
    #[test]
    fn airframe_strikes_both_destroyer_decks_in_their_own_frames() {
        for (ship, x) in [(1, -500.), (2, 500.)] {
            let mut f = flight(3.);
            let a = world(DVec3::new(x, 30., 40.), 2.9);
            f.position = world(DVec3::new(x, -5., 40.), 3.);
            let hit = aircraft(&f, a, rotation(2.9), 0.1).unwrap();
            assert_eq!(hit.ship, ship);
        }
    }
    #[test]
    fn collision_is_disabled_outside_carrier_scenarios() {
        let mut f = flight(1.);
        f.carrier.active = false;
        assert!(sweep(&f, origin(0.), origin(2.), 2., 10.).is_none());
    }
}
