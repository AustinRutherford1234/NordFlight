//! Bounded multi-point aircraft contact and swept collision tests against airport structures.
use crate::physics::*;
use bevy::math::DVec3;

pub fn destroy(f: &mut Flight, cause: &str, normal: DVec3) {
    if f.crashed {
        return;
    }
    f.crashed = true;
    f.autopilot = false;
    f.engine = false;
    f.throttle = 0.0;
    f.thrust = 0.0;
    f.impact_energy = 0.5 * f.mass() * f.velocity.length_squared();
    f.wreck_origin = f.position;
    f.wreck_velocity = f.velocity;
    f.wreck_age = 0.0;
    f.integrity = 0.0;
    f.damage.engine = 0.0;
    f.damage.engines=[0.;2];f.damage.nozzles=[0.;2];f.damage.lift_fan=0.;
    if f.fuel > 10.0 {
        f.damage.fire = 1.0;
    }
    f.damage.wings = [0.0; 2];
    f.damage.surface_state = [crate::combat::damage::SurfaceState::Detached; crate::aircraft::SURFACES];
    f.status = format!(
        "{} / IMPACT {:.0} kJ / R TO RESET",
        cause,
        f.impact_energy / 1000.0
    );
    let vn = f.velocity.dot(normal);
    if vn < 0.0 {
        f.velocity -= normal * vn * 1.12;
    }
    f.velocity *= 0.65;
    f.rates += DVec3::new(0.3, -0.15, 0.4);
}
pub fn wheel_clearance(f: &Flight) -> f64 {
    if f.gear_extension < 0.98 {
        return 0.6;
    }
    let up = f.up();
    if f.aircraft!=crate::aircraft::Aircraft::Gripen && f.vehicle==Vehicle::Gripen {return (0..3).map(|i|-(f.orientation*gear_pose(f,i,false).1).dot(up)+WHEEL_RADIUS).fold(0.6,f64::max);}
    wheels()
        .iter()
        .map(|p| -(f.orientation * *p).dot(up) + WHEEL_RADIUS)
        .fold(0.6, f64::max)
}
fn wheels() -> [DVec3; 3] {
    [
        DVec3::new(0.0, -1.3, -3.4),
        DVec3::new(-1.4, -1.3, 1.4),
        DVec3::new(1.4, -1.3, 1.4),
    ]
}
pub const GEAR_TRAVEL: f64 = 0.32;
pub const WHEEL_RADIUS: f64 = 0.33;
pub fn update_gear(f: &mut Flight, dt: f64) {
    let target = if f.gear { 1.0 } else { 0.0 };
    let mut remaining = dt;
    if (target - f.gear_extension).abs() > 1e-9 && (f.gear_phase == 0 || f.gear_phase == 3) {
        f.gear_phase = 1;
    }
    for _ in 0..5 {
        if remaining <= 1e-12 {
            break;
        }
        match f.gear_phase {
            1 => {
                let used = remaining.min((1.0 - f.gear_door) * 0.65);
                f.gear_door = (f.gear_door + used / 0.65).min(1.0);
                remaining -= used;
                if f.gear_door >= 1.0 - 1e-9 {
                    f.gear_door = 1.0;
                    f.gear_phase = 2;
                }
            }
            2 => {
                let used = remaining.min((target - f.gear_extension).abs() * 2.5);
                f.gear_extension += (target - f.gear_extension).signum() * used / 2.5;
                remaining -= used;
                if (f.gear_extension - target).abs() < 1e-9 {
                    f.gear_extension = target;
                    f.gear_phase = 3;
                }
            }
            3 => {
                // The wheel-cover doors close after both cycles. Separate
                // leg doors remain open around the deployed strut exits.
                let used = remaining.min(f.gear_door * 0.65);
                f.gear_door = (f.gear_door - used / 0.65).max(0.0);
                remaining -= used;
                if f.gear_door < 1e-9 {
                    f.gear_door = 0.0;
                    f.gear_phase = 0;
                }
            }
            _ => break,
        }
    }
    if f.gear_extension < 0.98 {
        f.gear_compression = [0.0; 3];
    }
}
/// Same retraction geometry is used by the renderer and physical wheel contacts.
pub fn gear_mount(index: usize) -> DVec3 {
    DVec3::new(
        if index == 0 {
            0.0
        } else if index == 1 {
            -0.40
        } else {
            0.40
        },
        if index == 0 { -0.09 } else { -0.15 },
        wheels()[index].z,
    )
}
pub fn gear_length(f: &Flight, index: usize, compressed: bool) -> f64 {
    let delta = wheels()[index] - gear_mount(index);
    let length = delta.length();
    length
        - if compressed {
            f.gear_compression[index] * length / (-delta.y)
        } else {
            0.0
        }
}
pub fn gear_pose(f: &Flight, index: usize, compressed: bool) -> (DVec3, DVec3, bevy::math::DQuat) {
    if f.aircraft!=crate::aircraft::Aircraft::Gripen && f.vehicle==Vehicle::Gripen {return crate::aircraft::visual::gear_pose(f,index,compressed);}
    let mount = gear_mount(index);
    let delta = wheels()[index] - mount;
    let lean = delta.x.atan2(-delta.y);
    let rotation =
        bevy::math::DQuat::from_rotation_x((1.0 - f.gear_extension) * std::f64::consts::FRAC_PI_2)
            * bevy::math::DQuat::from_rotation_z(lean * f.gear_extension.powi(2));
    (
        mount,
        mount + rotation * (-DVec3::Y * gear_length(f, index, compressed)),
        rotation,
    )
}
pub fn suspension(f: &mut Flight, dt: f64) -> (DVec3, DVec3) {
    let mut force = DVec3::ZERO;
    let mut torque = DVec3::ZERO;
    f.gear_compression = [0.0; 3];
    for i in 0..3 {
        if f.gear_health[i] <= 0.0 || f.gear_extension < 0.98 {
            continue;
        }
        let body = gear_pose(f, i, false).1;
        let arm = f.orientation * body;
        let p = f.position + arm;
        let gap = p.length() - RADIUS - terrain_height(p.x, p.z).max(0.0) - WHEEL_RADIUS;
        if gap > 0.0 {
            continue;
        }
        let velocity = f.velocity + f.orientation * f.rates.cross(body);
        let surface_normal = crate::terrain::normal(p);
        let vn = velocity.dot(surface_normal);
        let compression = (-gap).clamp(0.0, GEAR_TRAVEL);
        f.gear_compression[i] = compression;
        // Main oleos carry most of the weight; damping dissipates landing energy.
        let mass_scale=if f.aircraft==crate::aircraft::Aircraft::Gripen{1.}else{f.mass()/8300.};
        let stiffness = (if i == 0 { 65000.0 } else { 170000.0 })*mass_scale;
        let damping = (if i == 0 { 7000.0 } else { 16000.0 })*mass_scale;
        let bump_stop = (-gap - GEAR_TRAVEL).max(0.0) * 900000.0*mass_scale;
        let load = (compression * stiffness - vn * damping + bump_stop).clamp(0.0, 400000.0*mass_scale);
        force += surface_normal * load;
        torque += body.cross(f.orientation.conjugate() * (surface_normal * load));
        if load > 220000.0*mass_scale {
            f.gear_health[i] = (f.gear_health[i] - (load - 220000.0*mass_scale) * dt / (45000.0*mass_scale)).max(0.0);
        }
    }
    (
        force / f.mass(),
        torque / (f.aircraft.spec().inertia * (f.mass() / if f.aircraft==crate::aircraft::Aircraft::Gripen{8300.}else{f.aircraft.spec().empty+f.aircraft.spec().fuel*0.6})),
    )
}
pub fn wheel_contact(f: &Flight) -> bool {
    f.gear_extension >= 0.98
        && (0..3).any(|i| {
            if f.gear_health[i] <= 0. {
                return false;
            }
            let p = f.position + f.orientation * gear_pose(f, i, false).1;
            p.length() - RADIUS - terrain_height(p.x, p.z).max(0.) <= WHEEL_RADIUS
        })
}
/// Slab test gives the first time of impact, including segments longer than a building.
pub fn swept_box(a: DVec3, b: DVec3, min: DVec3, max: DVec3) -> Option<(f64, DVec3)> {
    let d = b - a;
    let mut enter = 0.0_f64;
    let mut leave = 1.0_f64;
    let mut normal = DVec3::ZERO;
    for axis in 0..3 {
        if d[axis].abs() < 1e-12 {
            if a[axis] < min[axis] || a[axis] > max[axis] {
                return None;
            }
            continue;
        }
        let mut t0 = (min[axis] - a[axis]) / d[axis];
        let mut t1 = (max[axis] - a[axis]) / d[axis];
        let mut n = DVec3::ZERO;
        n[axis] = -d[axis].signum();
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        if t0 > enter {
            enter = t0;
            normal = n;
        }
        leave = leave.min(t1);
        if enter > leave {
            return None;
        }
    }
    if normal == DVec3::ZERO {
        normal = -d.normalize_or_zero();
    }
    (enter <= 1.0 && leave >= 0.0).then_some((enter, normal))
}
fn boxes() -> [(DVec3, DVec3); 5] {
    let h = RUNWAY_HEIGHT;
    [
        (
            DVec3::new(222.5, h, 115.0),
            DVec3::new(297.5, h + 20.5, 205.0),
        ),
        (
            DVec3::new(222.5, h, 285.0),
            DVec3::new(297.5, h + 20.5, 375.0),
        ),
        (
            DVec3::new(222.5, h, 455.0),
            DVec3::new(297.5, h + 20.5, 545.0),
        ),
        (
            DVec3::new(222.5, h, 625.0),
            DVec3::new(297.5, h + 20.5, 715.0),
        ),
        (
            DVec3::new(118.5, h, -221.5),
            DVec3::new(141.5, h + 35.5, -198.5),
        ),
    ]
}
pub fn check_airframe(f: &mut Flight, previous: DVec3, previous_rotation: bevy::math::DQuat) {
    let _measure = crate::performance::span(12);
    let end = f.position;
    let angle = previous_rotation.angle_between(f.orientation);
    let steps = (angle / 0.025).ceil().clamp(1.0, 32.0) as usize;
    let radius=if f.aircraft==crate::aircraft::Aircraft::Gripen{9.}else{f.aircraft.spec().length*0.6};
    let margin = radius * (angle / steps as f64 * 0.5).sin().abs() + 0.015;
    let mut first_hit: Option<(f64, DVec3)> = None;
    let vegetation = crate::terrain::obstacles(previous, end);
    let airport = previous.length().min(end.length()) - RADIUS < RUNWAY_HEIGHT + 45.0+radius
        && previous.x.min(end.x) < 310.0+radius
        && previous.x.max(end.x) > 105.0-radius
        && previous.z.min(end.z) < 730.0+radius
        && previous.z.max(end.z) > -240.0-radius;
    let near_ground = f.agl() < 12.0
        || previous.length() - RADIUS - terrain_height(previous.x, previous.z).max(0.0) < 12.0;
    if !airport && !near_ground && vegetation.is_empty() {
        return;
    }
    // Every convex piece uses the same trajectory. Interpolate it once rather
    // than repeating quaternion slerps for every piece of the collision skin.
    let samples = steps.max(((end - previous).length() / 0.5).ceil().clamp(3.0, 64.0) as usize);
    let mut ground_samples = [(DVec3::ZERO, bevy::math::DQuat::IDENTITY); 65];
    if near_ground {
        for (k, pose) in ground_samples[..=samples].iter_mut().enumerate() {
            let time = k as f64 / samples as f64;
            *pose = (
                previous.lerp(end, time),
                previous_rotation.slerp(f.orientation, time),
            );
        }
    }
    if airport && f.vehicle == Vehicle::Gripen && f.gear_extension > 0.001 {
        // Tires and struts also collide with buildings throughout the retraction cycle.
        for i in 0..3 {
            if f.gear_health[i] <= 0.0 {
                continue;
            }
            let (mount, wheel, _) = gear_pose(f, i, true);
            for (body, radius) in [(wheel, WHEEL_RADIUS), ((mount + wheel) * 0.5, 0.12)] {
                let a = previous + previous_rotation * body;
                let b = end + f.orientation * body;
                let aa = DVec3::new(a.x, a.length() - RADIUS, a.z);
                let bb = DVec3::new(b.x, b.length() - RADIUS, b.z);
                for (min, max) in boxes() {
                    if let Some((time, n)) = swept_box(
                        aa,
                        bb,
                        min - DVec3::splat(radius),
                        max + DVec3::splat(radius),
                    ) && first_hit.is_none_or(|hit| time < hit.0)
                    {
                        first_hit = Some((time, n));
                    }
                }
            }
        }
    }
    for hull in crate::airframe::for_flight(f) {
        if !hull.enabled(f) {
            continue;
        }
        let (hinge, offset) = hull.pose(f);
        if airport || !vegetation.is_empty() {
            for step in 0..steps {
                let t0 = step as f64 / steps as f64;
                let t1 = (step + 1) as f64 / steps as f64;
                let q = previous_rotation.slerp(f.orientation, (t0 + t1) * 0.5);
                let a = previous.lerp(end, t0);
                let b = previous.lerp(end, t1);
                let origin = DVec3::new(a.x, a.length() - RADIUS, a.z) + q * offset;
                let delta = DVec3::new(b.x - a.x, b.length() - a.length(), b.z - a.z);
                for (min, max) in boxes()
                    .into_iter()
                    .filter(|_| airport)
                    .chain(vegetation.iter().copied())
                {
                    if let Some((t, n)) =
                        hull.sweep_box(origin, delta, q * hinge, DVec3::ZERO, min, max, margin)
                    {
                        let time = t0 + (t1 - t0) * t;
                        if first_hit.is_none_or(|hit| time < hit.0) {
                            first_hit = Some((time, n));
                        }
                    }
                }
            }
        }
        if near_ground {
            // Interpolate both position and attitude, so rolling wing/fin strikes count.
            for (k, &(center, q)) in ground_samples[..=samples].iter().enumerate() {
                let time = k as f64 / samples as f64;
                if above_flat_ground(hull, center, q, hinge, offset) {
                    continue;
                }
                for vertex in &hull.vertices {
                    let p = center + q * (hinge * *vertex + offset);
                    if p.length() - RADIUS < terrain_height(p.x, p.z).max(0.0) + 0.015
                        && first_hit.is_none_or(|hit| time < hit.0)
                    {
                        first_hit = Some((time, p.normalize()));
                    }
                }
            }
        }
    }
    if let Some((time, normal)) = first_hit {
        f.position = previous.lerp(end, (time - 0.0001).max(0.0));
        f.orientation = previous_rotation.slerp(f.orientation, time);
        destroy(
            f,
            if airport && normal.y.abs() < 0.9 {
                "STRUCTURE COLLISION"
            } else {
                "AIRFRAME COLLISION"
            },
            normal,
        );
    }
}
fn above_flat_ground(
    hull: &crate::airframe::Hull,
    center: DVec3,
    q: bevy::math::DQuat,
    hinge: bevy::math::DQuat,
    offset: DVec3,
) -> bool {
    let (min, max) = hull.bounds();
    let half = (max - min) * 0.5;
    let rotation = q * hinge;
    let middle = center + q * offset + rotation * ((min + max) * 0.5);
    let extent = (rotation * (DVec3::X * half.x)).abs()
        + (rotation * (DVec3::Y * half.y)).abs()
        + (rotation * (DVec3::Z * half.z)).abs();
    // The flattened airfield has a global upper bound, even if a lake polygon
    // touches it (lake bed/water are lower). Outside it, retain every old test.
    if middle.x.abs() + extent.x >= 310. || middle.z.abs() + extent.z >= 1850. {
        return false;
    }
    middle.dot(center.normalize()) - extent.dot(center.normalize().abs())
        > RADIUS + RUNWAY_HEIGHT + 0.015 + 1e-6
}
pub fn simulate_wreck(f: &mut Flight, dt: f64) {
    f.elapsed += dt;
    f.wreck_age += dt;
    let up = f.up();
    f.velocity -= up * (MU / f.position.length_squared()) * dt;
    f.position += f.velocity * dt;
    let ground = terrain_height(f.position.x, f.position.z).max(0.0) + 0.65;
    if f.altitude() < ground {
        f.position = up * (RADIUS + ground);
        let vn = f.velocity.dot(up);
        if vn < 0.0 {
            f.velocity -= up * vn * 1.08;
        }
        let tangent = f.velocity - up * f.velocity.dot(up);
        let speed = tangent.length();
        f.velocity -= tangent.normalize_or_zero() * (speed.min(dt * 9.81 * 0.7));
        f.rates *= (-dt * 3.0).exp();
    }
    f.orientation = (f.orientation * bevy::math::DQuat::from_scaled_axis(f.rates * dt)).normalize();
    f.airspeed = f.velocity.length();
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejected_ground_bounds_never_hide_a_vertex_strike() {
        for height in [0.7, 1.3, 1.8, 4., 10.] {
            for bank in [-0.7, 0., 0.7] {
                for pitch in [-0.4, 0., 0.4] {
                    let mut f = Flight::runway();
                    f.position = surface_point(0., 0., RUNWAY_HEIGHT + height);
                    f.orientation *=
                        bevy::math::DQuat::from_euler(bevy::math::EulerRot::XYZ, pitch, 0., bank);
                    for hull in crate::airframe::for_flight(&f) {
                        let (hinge, offset) = hull.pose(&f);
                        if above_flat_ground(hull, f.position, f.orientation, hinge, offset) {
                            assert!(hull.vertices.iter().all(|v| {
                                let p = f.position + f.orientation * (hinge * *v + offset);
                                p.length() - RADIUS > terrain_height(p.x, p.z).max(0.) + 0.015
                            }));
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn wheel_doors_open_before_motion_and_close_after_both_cycles() {
        let mut f = Flight::runway();
        f.gear_extension = 0.0;
        f.gear_door = 0.0;
        f.gear = true;
        update_gear(&mut f, 0.4);
        assert_eq!(f.gear_extension, 0.0);
        assert!(f.gear_door > 0.5 && f.gear_door < 1.0);
        for _ in 0..400 {
            update_gear(&mut f, 0.01);
            if f.gear_extension > 0.0 && f.gear_extension < 1.0 {
                assert_eq!(f.gear_door, 1.0);
            }
        }
        assert_eq!(f.gear_extension, 1.0);
        assert_eq!(f.gear_door, 0.0);
        f.gear = false;
        update_gear(&mut f, 4.0);
        assert_eq!(f.gear_extension, 0.0);
        assert_eq!(f.gear_door, 0.0);
    }
    #[test]
    fn gear_cycle_is_timed_reversible_and_stowed_gear_cannot_support_weight() {
        let mut f = Flight::runway();
        f.gear = false;
        update_gear(&mut f, 1.90);
        assert!((f.gear_extension - 0.5).abs() < 1e-9);
        f.gear = true;
        update_gear(&mut f, 0.625);
        assert!((f.gear_extension - 0.75).abs() < 1e-9);
        f.gear = false;
        update_gear(&mut f, 3.0);
        assert_eq!(f.gear_extension, 0.0);
        assert_eq!(suspension(&mut f, 0.01).0, DVec3::ZERO);
        f.gear = true;
        update_gear(&mut f, 3.8);
        assert_eq!(f.gear_extension, 1.0);
    }
    #[test]
    fn oleos_compress_and_damping_opposes_vertical_motion() {
        let mut f = Flight::runway();
        f.position = surface_point(0.0, 0.0, RUNWAY_HEIGHT + 1.43);
        f.velocity = DVec3::ZERO;
        let resting = suspension(&mut f, 0.001).0.dot(f.up());
        assert!(
            f.gear_compression
                .iter()
                .all(|x| *x > 0.18 && *x < GEAR_TRAVEL)
        );
        f.velocity = -f.up() * 1.0;
        let descending = suspension(&mut f, 0.001).0.dot(f.up());
        f.velocity = f.up() * 1.0;
        let ascending = suspension(&mut f, 0.001).0.dot(f.up());
        assert!(descending > resting && resting > ascending && ascending >= 0.0);
    }
    #[test]
    fn rolling_airframe_hits_ground_before_its_center() {
        let mut f = Flight::runway();
        f.position = surface_point(0.0, 0.0, RUNWAY_HEIGHT + 2.0);
        let previous = f.position;
        let old = f.orientation;
        f.orientation = bevy::math::DQuat::from_rotation_z(0.8);
        check_airframe(&mut f, previous, old);
        assert!(f.crashed);
    }
    #[test]
    fn short_drop_settles_on_springs_without_airframe_damage() {
        let mut f = Flight::runway();
        f.landed = false;
        f.position = surface_point(0.0, 0.0, RUNWAY_HEIGHT + 1.85);
        let mut e = Environment::default();
        e.wind_enabled = false;
        let mut compressed = 0.0_f64;
        for _ in 0..2400 {
            simulate(&mut f, &e, 1.0 / 120.0);
            compressed = compressed.max(f.gear_compression[1]);
            assert!(!f.crashed, "{}", f.status);
        }
        assert!(compressed > 0.10);
        assert!(f.velocity.dot(f.up()).abs() < 0.1);
        assert!(f.gear_health.iter().all(|h| *h > 0.99));
        assert!(f.gear_compression[1] > 0.05);
    }
    #[test]
    fn swept_hit_does_not_tunnel() {
        let hit = swept_box(
            DVec3::new(-100.0, 0.0, 0.0),
            DVec3::new(100.0, 0.0, 0.0),
            DVec3::splat(-1.0),
            DVec3::splat(1.0),
        )
        .unwrap();
        assert!((hit.0 - 0.495).abs() < 1e-9);
        assert_eq!(hit.1, -DVec3::X);
    }
    #[test]
    fn swept_miss() {
        assert!(
            swept_box(
                DVec3::new(-2.0, 4.0, 0.0),
                DVec3::new(2.0, 4.0, 0.0),
                DVec3::splat(-1.0),
                DVec3::splat(1.0)
            )
            .is_none()
        );
    }
    #[test]
    fn hangar_collision() {
        let mut f = Flight::approach();
        f.position = surface_point(260.0, 160.0, RUNWAY_HEIGHT + 10.0);
        f.velocity = DVec3::X * 100.0;
        let previous = surface_point(180.0, 160.0, RUNWAY_HEIGHT + 10.0);
        let rotation = f.orientation;
        check_airframe(&mut f, previous, rotation);
        assert!(f.crashed);
        assert!(f.impact_energy > 1e6);
    }
    #[test]
    fn wreck_continues_after_impact() {
        let mut f = Flight::approach();
        destroy(&mut f, "TEST", DVec3::Y);
        let before = f.position;
        simulate_wreck(&mut f, 0.01);
        assert_ne!(f.position, before);
        assert!(f.wreck_age > 0.0);
    }
}
