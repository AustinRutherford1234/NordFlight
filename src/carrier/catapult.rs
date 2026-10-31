//! Launch geometry measured from Grant's carrier deck. Joint forces constrain
//! the launch attachment, with point velocity and inertia-aware damping.
use super::*;
use crate::aircraft::hornet::{Loads, mass};
const MODEL_SCALE: f64 = 333. / 303.842285;
// Centres of the imported Object_6 bow-port rail borders. Blender source XY
// becomes ship (-X,+Z) after the same centering/rotation as the visual model.
pub const X: f64 = (222.12742 - 226.96022033691406) * MODEL_SCALE;
pub const HEIGHT: f64 = 21.21270751953125 * MODEL_SCALE;
pub const START: DVec3 = DVec3::new(X, HEIGHT, (51.34455 - 79.04874420166016) * MODEL_SCALE);
pub const END: DVec3 = DVec3::new(X, HEIGHT, (51.34455 - 199.36740112304688) * MODEL_SCALE);
pub const LENGTH: f64 = START.z - END.z;
pub const TARGET_SPEED: f64 = 78.;
const LATERAL_K: f64 = 350000.;
const HOLD_K: f64 = 2000000.;
pub fn point(stroke: f64) -> DVec3 {
    START - DVec3::Z * stroke.clamp(0., LENGTH)
}
pub fn park_position() -> DVec3 {
    let p = START - LAUNCH_POINT;
    DVec3::new(p.x, DECK, p.z)
}
pub fn attachment(f: &Flight) -> DVec3 {
    ship_relative(f,0,f.position + f.orientation * LAUNCH_POINT, f.elapsed)
}
pub fn stroke(f: &Flight) -> f64 {
    START.z - attachment(f).z
}
fn point_mass(m: f64, inverse: bevy::math::DMat3, arm: DVec3, axis: DVec3) -> f64 {
    1. / (1. / m + (inverse * arm.cross(axis)).cross(arm).dot(axis)).max(1e-9)
}
pub fn loads(f: &mut Flight, dt: f64) -> Loads {
    let mut result = Loads::default();
    if !f.carrier.attached && f.carrier.launch.is_none() {
        return result;
    }
    let (cg, inertia) = mass::properties(f);
    let inverse = inertia.inverse();
    let arm = LAUNCH_POINT - cg;
    let basis = ship_rotation(f,0,f.elapsed);
    let body = f.orientation;
    let to_body = |v: DVec3| body.conjugate() * (basis * v);
    let p = attachment(f);
    let v = basis.conjugate() * (f.velocity + body * f.rates.cross(arm) - ship_velocity(f,0,f.position,f.elapsed));
    let lateral_damping =
        2. * 1.1 * (LATERAL_K * point_mass(f.mass(), inverse, arm, to_body(DVec3::X))).sqrt();
    let lateral = (-LATERAL_K * (p.x - X) - lateral_damping * v.x).clamp(-250000., 250000.);
    if f.carrier.attached {
        let damping =
            2. * 1.1 * (HOLD_K * point_mass(f.mass(), inverse, arm, to_body(DVec3::Z))).sqrt();
        let longitudinal = (-HOLD_K * (p.z - START.z) - damping * v.z).clamp(-500000., 500000.);
        result.add(to_body(DVec3::new(lateral, 0., longitudinal)), arm);
    }
    if let Some(age) = f.carrier.launch {
        let distance = stroke(f);
        f.carrier.launch_stroke = distance.clamp(0., LENGTH);
        if distance >= LENGTH || age > 8. || !f.engine || f.throttle < 0.5 || (p.x - X).abs() > 1.5
        {
            f.carrier.launch = None;
            f.status = if distance >= LENGTH {
                "CATAPULT RELEASE / S ROTATE / G GEAR UP"
            } else {
                "CATAPULT ABORT / MANUAL CONTROL"
            }
            .into();
            return result;
        }
        f.carrier.launch = Some(age + dt);
        let engine_forward =
            (basis.conjugate() * (body * f.hornet.diagnostic.engine.force)).dot(DVec3::NEG_Z);
        // Normalize the spatial ramp/taper area, rather than losing launch
        // energy when the visible rail length or endpoint changes.
        let shape =
            ((distance + 0.5) / 5.).clamp(0.1, 1.) * ((LENGTH - distance) / 8.).clamp(0., 1.);
        let normalization = LENGTH / (LENGTH - 6.5);
        let pull = (f.mass() * TARGET_SPEED.powi(2) / (2. * LENGTH) - engine_forward)
            .clamp(100000., 800000.)
            * shape
            * normalization;
        let pull = pull.min(800000.);
        result.add(to_body(DVec3::new(lateral, -pull * 0.38, -pull)), arm);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default, Debug)]
    struct Metrics {
        bar: f64,
        cg: f64,
        yaw: f64,
        release_speed: f64,
        time: f64,
        reversals: usize,
    }
    fn run(fuel: f64, tanks: bool, perturbed: bool, hz: usize, wind: bool, name: &str) -> Metrics {
        let mut f = scenario(true);
        f.fuel = fuel;
        if tanks {
            crate::aircraft::hornet::stores::fit(&mut f);
        }
        f.throttle = 1.;
        let e = Environment {
            wind_enabled: wind,
            ..Default::default()
        };
        for _ in 0..hz * 4 {
            simulate(&mut f, &e, 1. / hz as f64);
        }
        if perturbed {
            f.orientation *= DQuat::from_rotation_y(3f64.to_radians());
            f.velocity += rotation(f.elapsed) * DVec3::X;
        }
        launch(&mut f);
        assert!(f.carrier.launch.is_some());
        let start = f.elapsed;
        let mut metrics = Metrics::default();
        let mut old = 0.;
        let mut rows = String::from("time,cg_x,cg_z,bar_x,bar_z,yaw_deg,yaw_rate,side_velocity\n");
        for _ in 0..hz * 10 {
            let p = relative(f.position, f.elapsed);
            let bar = attachment(&f);
            let direction = rotation(f.elapsed).conjugate() * f.forward();
            let yaw = direction.x.atan2(-direction.z).to_degrees();
            let v = rotation(f.elapsed).conjugate() * (f.velocity - velocity(f.elapsed));
            metrics.bar = metrics.bar.max((bar.x - X).abs());
            metrics.cg = metrics.cg.max((p.x - X).abs());
            metrics.yaw = metrics.yaw.max(yaw.abs());
            if v.x.abs() > 0.01 && old * v.x < 0. {
                metrics.reversals += 1;
            }
            if v.x.abs() > 0.01 {
                old = v.x;
            }
            rows += &format!(
                "{},{},{},{},{},{},{},{}\n",
                f.elapsed - start,
                p.x,
                p.z,
                bar.x,
                bar.z,
                yaw,
                f.rates.y,
                v.x
            );
            simulate(&mut f, &e, 1. / hz as f64);
            if f.carrier.launch.is_none() {
                metrics.release_speed = (f.velocity - velocity(f.elapsed)).length();
                metrics.time = f.elapsed - start;
                break;
            }
        }
        std::fs::create_dir_all("tests/carrier-catapult").unwrap();
        std::fs::write(format!("tests/carrier-catapult/{name}.csv"), rows).unwrap();
        assert!(
            !f.crashed && metrics.time > 0. && f.carrier.launch.is_none(),
            "{name}: {} {metrics:?}",
            f.status
        );
        assert_eq!(
            f.status, "CATAPULT RELEASE / S ROTATE / G GEAR UP",
            "{name}: {metrics:?}"
        );
        assert!(
            metrics.release_speed > 70. && metrics.release_speed < 86.,
            "{name}: {metrics:?}"
        );
        assert!(
            (stroke(&f) - LENGTH).abs() < 2.,
            "{name}: release outside rail endpoint"
        );
        println!("TRACK {name}: {metrics:?}");
        metrics
    }
    #[test]
    fn spawn_and_joint_share_the_visible_rail() {
        let f = scenario(true);
        let p = attachment(&f);
        assert!((p.x - X).abs() < 1e-7);
        assert!(stroke(&f).abs() < 1e-7);
        assert!(inside(START) && inside(END));
        assert!(X < 0. && START.z < -25. && END.z < -160.);
        assert!((point(LENGTH) - END).length() < 1e-9);
    }
    #[test]
    fn point_damping_uses_angular_velocity_without_imposing_motion() {
        let mut f = scenario(true);
        f.throttle = 1.;
        launch(&mut f);
        let (cg, _) = mass::properties(&f);
        let arm = LAUNCH_POINT - cg;
        f.rates = DVec3::Y * 0.2;
        f.velocity = velocity(f.elapsed) - f.orientation * f.rates.cross(arm);
        let original = (f.position, f.velocity, f.orientation, f.rates);
        let l = loads(&mut f, 1. / 240.);
        assert!(
            l.force.x.abs() < 1e-6,
            "damping must see stationary attachment rather than moving CG: {:?}",
            l.force
        );
        assert_eq!(original, (f.position, f.velocity, f.orientation, f.rates));
    }
    #[test]
    fn light_heavy_tanks_and_crosswind_track_straight() {
        for (fuel, tanks, wind, name) in [
            (1000., false, false, "light"),
            (6000., false, false, "heavy"),
            (6000., true, false, "tanks"),
            (6000., true, true, "crosswind"),
        ] {
            let m = run(fuel, tanks, false, 240, wind, name);
            assert!(m.bar < 0.04 && m.yaw < 0.6, "{name}: {m:?}");
        }
    }
    #[test]
    fn alignment_disturbance_does_not_grow_into_a_launch_snake() {
        let m = run(3900., false, true, 240, false, "perturbed");
        assert!(m.bar < 0.3 && m.cg < 0.35 && m.yaw < 4.5, "{m:?}");
    }
    #[test]
    fn frame_partition_preserves_launch_speed_and_duration() {
        let mut values = Vec::new();
        for hz in [60, 120, 240] {
            values.push(run(6000., true, false, hz, false, &format!("hz-{hz}")));
        }
        let reference = &values[2];
        for m in &values[..2] {
            assert!((m.release_speed / reference.release_speed - 1.).abs() < 0.02);
            assert!((m.time / reference.time - 1.).abs() < 0.02);
        }
    }
}
