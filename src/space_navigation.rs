//! Shared patched-conic prediction and a manual-transfer planning aid.
use super::*;
use crate::{map::Maneuver, orbit::Orbit};
#[derive(Default)]
pub struct Prediction {
    pub points: Vec<(DVec3, f64, Body)>,
    pub encounters: Vec<(Body, f64, DVec3)>,
    pub impact: Option<(Body, f64)>,
    pub closest_moon: f64,
}
/// Short-way, zero-revolution Lambert solution. This is an impulse-planning aid;
/// finite burns remain manual and must be corrected against the live prediction.
pub fn lambert(r1: DVec3, r2: DVec3, mu: f64, dt: f64, normal: DVec3) -> Option<DVec3> {
    let (a, b) = (r1.length(), r2.length());
    let cosine = r1.dot(r2) / (a * b);
    let sine = (1. - cosine * cosine).max(0.).sqrt();
    let aa = sine
        * (a * b / (1. - cosine).max(1e-12)).sqrt()
        * if r1.cross(r2).dot(normal) < 0. {
            -1.
        } else {
            1.
        };
    if aa.abs() < 1e-5 {
        return None;
    }
    let evaluate = |z: f64| -> Option<(f64, f64)> {
        let (c, s) = stumpff(z);
        if c <= 0. {
            return None;
        }
        let y = a + b + aa * (z * s - 1.) / c.sqrt();
        if y <= 0. {
            return None;
        }
        Some((((y / c).powf(1.5) * s + aa * y.sqrt()) / mu.sqrt(), y))
    };
    let mut lo = -4. * std::f64::consts::PI.powi(2);
    let mut hi = 4. * std::f64::consts::PI.powi(2) - 1e-6;
    for _ in 0..120 {
        let z = (lo + hi) * 0.5;
        match evaluate(z) {
            Some((t, _)) if t > dt => hi = z,
            _ => lo = z,
        };
    }
    let (_, y) = evaluate((lo + hi) * 0.5)?;
    let ff = 1. - y / a;
    let g = aa * (y / mu).sqrt();
    let v = (r2 - r1 * ff) / g;
    v.is_finite().then_some(v)
}
pub fn moon_transfer(f: &Flight) -> Option<Maneuver> {
    if f.space.body != Body::Earth {
        return None;
    }
    let o = Orbit::from_flight(f);
    let r = relative(f).0;
    let n = o.angular_momentum.normalize();
    let x = if o.eccentricity > 1e-6 {
        o.eccentricity_vector.normalize()
    } else {
        r.normalize()
    };
    let current = r.dot(n.cross(x)).atan2(r.dot(x));
    let mut node = Maneuver {
        anomaly: current + 0.4,
        dv: DVec3::ZERO,
        reference: x,
        epoch: f.elapsed,
    };
    let eta = node.eta(f)?;
    let (p, v, _) = node.states(f)?;
    let flight_time = 3.5 * 86400.;
    let mp = moon_inertial(f.elapsed + eta + flight_time).0;
    let radial = p.normalize();
    let tangent = n.cross(radial);
    let make = |offset: f64| -> Option<Maneuver> {
        let target = earth_rotation(f.elapsed).conjugate()
            * (mp + mp.normalize().cross(n).normalize() * offset);
        let velocity = lambert(p, target, MU, flight_time, n)?;
        let dv = velocity - v;
        let mut planned = node;
        planned.dv = DVec3::new(dv.dot(tangent), dv.dot(radial), dv.dot(n));
        Some(planned)
    };
    // Target a safe periapsis in the gameplay model, not the Moon's center.
    let (mut low, mut high) = (1_000_000., 24_000_000.);
    for _ in 0..16 {
        let mid = (low + high) * 0.5;
        let trial = make(mid)?;
        let predicted = predict(f, Some(trial));
        if predicted.closest_moon < 120000. {
            low = mid;
        } else {
            high = mid;
        }
    }
    node = make(high)?;
    Some(node)
}
pub fn predict(f: &Flight, node: Option<Maneuver>) -> Prediction {
    let mut out=Prediction::default();
    predict_into(f,node,&mut out);
    out
}
pub fn predict_into(f:&Flight,node:Option<Maneuver>,out:&mut Prediction){
    out.points.clear();out.encounters.clear();out.impact=None;out.closest_moon=f64::INFINITY;
    out.points.reserve(768);
    let q = earth_rotation(f.elapsed);
    let mut time = f.elapsed;
    let mut body = f.space.body;
    let (mut p, mut v) = if let Some(node) = node {
        if let Some((r, v, _)) = node.states(f) {
            time += node.eta(f).unwrap_or(0.);
            let rel = r - center(body, f.elapsed);
            let (c, cv) = if body == Body::Moon {
                let (a, b, _, _) = moon_inertial(time);
                (a, b)
            } else {
                (DVec3::ZERO, DVec3::ZERO)
            };
            (q * rel + c, q * v + cv)
        } else {
            (
                q * f.position,
                q * (f.velocity + rotation().cross(f.position)),
            )
        }
    } else {
        (
            q * f.position,
            q * (f.velocity + rotation().cross(f.position)),
        )
    };
    for _ in 0..768 {
        let mp = moon_inertial(time);
        let next_body = if p.distance(mp.0) < MOON_SOI {
            Body::Moon
        } else {
            Body::Earth
        };
        if next_body != body {
            out.encounters
                .push((next_body, time - f.elapsed, q.conjugate() * p));
            body = next_body;
        }
        let (c, cv) = if body == Body::Moon {
            (mp.0, mp.1)
        } else {
            (DVec3::ZERO, DVec3::ZERO)
        };
        let r = p - c;
        let rv = v - cv;
        out.points.push((q.conjugate() * p, time, body));
        out.closest_moon = out.closest_moon.min(p.distance(mp.0) - MOON_RADIUS);
        if r.length() < body.radius() + if body == Body::Earth { 60000. } else { 1000. } {
            out.impact = Some((body, time - f.elapsed));
            break;
        }
        let boundary = (p.distance(mp.0) - MOON_SOI).abs() / (v - mp.1).length().max(1.);
        let dt = (r.length() / rv.length().max(10.) * 0.04)
            .clamp(0.25, 1800.)
            .min(boundary * 0.2 + 1.);
        let Some((r1, v1)) = propagate(r, rv, body.mu(), dt) else {
            break;
        };
        time += dt;
        let (c1, vcenter) = if body == Body::Moon {
            let (a, b, _, _) = moon_inertial(time);
            (a, b)
        } else {
            (DVec3::ZERO, DVec3::ZERO)
        };
        p = r1 + c1;
        v = v1 + vcenter;
        if time - f.elapsed > 7. * 86400. {
            break;
        }
    }
}
pub fn plot_point(
    point: (DVec3, f64, Body),
    epoch: f64,
    now: f64,
    moon_focus: bool,
) -> Option<DVec3> {
    let (p, t, body) = point;
    let q0 = earth_rotation(epoch);
    let q1 = earth_rotation(now).conjugate();
    if moon_focus {
        if body != Body::Moon {
            return None;
        }
        Some(moon(now).0 + q1 * (q0 * p - moon_inertial(t).0))
    } else {
        Some(q1 * q0 * p)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prediction_reuses_storage_without_retaining_previous_encounters_or_impact() {
        let f=Flight::spacecraft();let expected=predict(&f,None);
        let mut reused=Prediction::default();
        predict_into(&f,None,&mut reused);
        let capacity=reused.points.capacity();
        reused.impact=Some((Body::Earth,-1.));reused.encounters.push((Body::Moon,-1.,DVec3::ZERO));
        predict_into(&f,None,&mut reused);
        assert_eq!(reused.points,expected.points);assert_eq!(reused.encounters,expected.encounters);
        assert_eq!(reused.impact,expected.impact);assert_eq!(reused.closest_moon,expected.closest_moon);
        assert_eq!(capacity,reused.points.capacity());
    }
    #[test]
    fn moon_focused_prediction_stays_centered_on_the_moving_moon() {
        let f = scenario(9);
        let prediction = predict(&f, None);
        for point in prediction.points.iter().step_by(13) {
            let p = plot_point(*point, f.elapsed, 100., true).unwrap();
            assert!((p.distance(moon(100.).0) - MOON_RADIUS - 100000.).abs() < 0.01);
        }
    }
    #[test]
    fn lambert_endpoint_and_manual_transfer_budget() {
        let f = Flight::spacecraft();
        let node = moon_transfer(&f).unwrap();
        let eta = node.eta(&f).unwrap();
        let (p, v, _) = node.states(&f).unwrap();
        let (pos, _) = propagate(p, v, MU, 3.5 * 86400.).unwrap();
        let mp =
            earth_rotation(f.elapsed).conjugate() * moon_inertial(f.elapsed + eta + 3.5 * 86400.).0;
        assert!(
            pos.distance(mp) > 1_000_000. && pos.distance(mp) < 24_000_000.,
            "{}",
            pos.distance(mp)
        );
        assert!(
            node.dv.length() < delta_v(&f),
            "dv {} available {}",
            node.dv.length(),
            delta_v(&f)
        );
        let prediction = predict(&f, Some(node));
        assert!(
            (100000. ..140000.).contains(&prediction.closest_moon),
            "{}",
            prediction.closest_moon
        );
        assert!(
            prediction
                .encounters
                .iter()
                .any(|(b, _, _)| *b == Body::Moon)
        );
    }
}
