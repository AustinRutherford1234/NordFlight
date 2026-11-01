//! Force-limited, estimated naval SAM plant. Sensor estimates only drive guidance.
use super::*;
#[derive(Clone, Debug)]
pub struct FlightState {
    pub acceleration: DVec3,
    pub best_range: f64,
    pub opening: f64,
    pub no_closure: f64,
}
impl Default for FlightState {
    fn default() -> Self {
        Self {
            acceleration: DVec3::ZERO,
            best_range: f64::INFINITY,
            opening: 0.,
            no_closure: 0.,
        }
    }
}
#[derive(Clone, Copy)]
struct Spec {
    wet: f64,
    dry: f64,
    area: f64,
    wing: f64,
    max_g: f64,
    boost: f64,
    sustain: f64,
    boost_time: f64,
    burn: f64,
    life: f64,
    cruise: f64,
}
fn spec(kind: Kind) -> Spec {
    match kind {
        Kind::Standard => Spec {
            wet: 700.,
            dry: 450.,
            area: 0.092,
            wing: 0.8,
            max_g: 30.,
            boost: 100000.,
            sustain: 28000.,
            boost_time: 4.,
            burn: 14.,
            life: 100.,
            cruise: 1050.,
        },
        _ => Spec {
            wet: 280.,
            dry: 180.,
            area: 0.051,
            wing: 0.42,
            max_g: 40.,
            boost: 35000.,
            sustain: 20000.,
            boost_time: 2.,
            burn: 9.,
            life: 50.,
            cruise: 950.,
        },
    }
}
fn mass(s: Spec, t: f64) -> f64 {
    s.wet - (s.wet - s.dry) * (t / s.burn).clamp(0., 1.)
}
fn authority(s: Spec, p: DVec3, v: DVec3, t: f64) -> f64 {
    let rho = atmosphere((p.length() - RADIUS).max(0.)).2;
    (0.5 * rho * v.length_squared() * s.wing * 2.5 / mass(s, t)).min(s.max_g * 9.81)
}
pub(super) fn available_g(m: &Round) -> f64 {
    authority(spec(m.kind), m.position, m.velocity, m.age) / 9.81
}
/// Positive solution of |r+v*t|=speed*t; none for an unreachable constant-speed target.
fn lead_time(r: DVec3, v: DVec3, speed: f64) -> Option<f64> {
    let a = v.length_squared() - speed * speed;
    let b = 2. * r.dot(v);
    let c = r.length_squared();
    if a.abs() < 1e-8 {
        return (b < -1e-8).then(|| -c / b);
    }
    let d = b * b - 4. * a * c;
    if d < 0. {
        return None;
    }
    [(-b - d.sqrt()) / (2. * a), (-b + d.sqrt()) / (2. * a)]
        .into_iter()
        .filter(|t| *t > 0. && t.is_finite())
        .min_by(f64::total_cmp)
}
fn command(p: DVec3, v: DVec3, target: DVec3, tv: DVec3, cruise: f64) -> DVec3 {
    let speed = v.length().max(1.);
    let direction = v / speed;
    let r = target - p;
    let range = r.length().max(1.);
    let relative = tv - v;
    let closing = -relative.dot(r / range);
    let t = lead_time(r, tv, speed.max(cruise * 0.7))
        .unwrap_or(range / cruise)
        .min(75.);
    let aim = (r + tv * t).normalize_or(direction);
    let capture = (aim - direction * aim.dot(direction)) * speed * 1.5;
    let weight = ((direction.angle_between(aim) - 0.05) / 0.2).clamp(0., 1.);
    let los_rate = r.cross(relative) / (range * range);
    let pn = los_rate.cross(direction) * (4. * closing.max(0.));
    pn + capture * weight
}
pub(super) fn advance(m: &mut Round, track: Option<(DVec3, DVec3)>, dt: f64) {
    let s = spec(m.kind);
    let steps = (dt * 240.).ceil().max(1.) as usize;
    let h = dt / steps as f64;
    for k in 0..steps {
        let age = m.age - dt + (k + 1) as f64 * h;
        let speed = m.velocity.length().max(1.);
        let direction = m.velocity.normalize_or(m.position.normalize());
        let up = m.position.normalize();
        let target = track.map(|(p, v)| (p + v * ((k + 1) as f64 * h - dt), v));
        let request = if matches!(m.source, TargetId::Ship(_)) && age < 0.6 {
            (up - direction * up.dot(direction)) * speed * 2.
        } else if let Some((p, v)) = target {
            command(m.position, m.velocity, p, v, s.cruise)
        } else {
            DVec3::ZERO
        };
        let gravity = up * 9.81;
        let normal_gravity = gravity - direction * gravity.dot(direction);
        let limit = authority(s, m.position, m.velocity, age);
        let requested = if target.is_some() {
            request + normal_gravity
        } else {
            DVec3::ZERO
        };
        m.sam.acceleration +=
            (requested.clamp_length_max(limit) - m.sam.acceleration) * (1. - (-h / 0.07).exp());
        let lift = (m.sam.acceleration - direction * m.sam.acceleration.dot(direction))
            .clamp_length_max(limit);
        m.sam.acceleration = lift;
        let (_, _, rho, sound) = atmosphere((m.position.length() - RADIUS).max(0.));
        let q = 0.5 * rho * speed * speed;
        let mach = speed / sound;
        let cd = 0.32 + 0.28 * (-((mach - 1.1) / 0.32).powi(2)).exp();
        let weight = mass(s, age);
        let drag = q * s.area * cd / weight
            + 0.06 * (weight * lift.length()).powi(2) / (q.max(1.) * s.wing * weight);
        let thrust = if age < s.boost_time {
            s.boost
        } else if age < s.burn {
            s.sustain
        } else {
            0.
        };
        let normal = lift - normal_gravity;
        let axis = direction.cross(normal).normalize_or_zero();
        let angle = normal.length() * h / speed;
        let next = if axis.length_squared() > 0.5 {
            DQuat::from_axis_angle(axis, angle) * direction
        } else {
            direction
        };
        let next_speed = (speed + (thrust / weight - drag - gravity.dot(direction)) * h).max(0.);
        let next_velocity = next * next_speed;
        m.position += (m.velocity + next_velocity) * (h * 0.5);
        m.velocity = next_velocity;
        m.visual.gnc.commanded_acceleration = lift;
        m.visual.gnc.actual_g = lift.length() / 9.81;
        if let Some((p, v)) = target {
            let r = p - m.position;
            let range = r.length();
            let closing = -(v - m.velocity).dot(r.normalize_or_zero());
            m.sam.best_range = m.sam.best_range.min(range);
            if age > s.burn + 3. && range > 2000. && closing <= 0. {
                m.sam.no_closure += h;
            } else {
                m.sam.no_closure = (m.sam.no_closure - h).max(0.);
            }
            if m.sam.best_range < 2000. && range > m.sam.best_range + 100. && closing < -20. {
                m.sam.opening += h;
            } else {
                m.sam.opening = (m.sam.opening - h).max(0.);
            }
        }
        let spent =
            age > s.burn + 2. && (next_speed < 170. || (limit < 2. * 9.81 && next_speed < 300.));
        if age > s.life
            || spent
            || m.sam.opening > 2.5
            || m.sam.no_closure > 8.
            || !m.velocity.is_finite()
            || !m.position.is_finite()
        {
            m.alive = false;
            m.visual.gnc.reason = if spent || m.sam.no_closure > 8. {
                "ENERGY DEPLETED"
            } else if m.sam.opening > 2.5 {
                "MISSED INTERCEPT"
            } else {
                "FLIGHT LIMIT"
            };
            break;
        }
    }
}
/// Opt-in native inspection uses the same supported-target weapon path as tests.
pub(super) fn prepare_review(mode: &str, f: &mut Flight, c: &mut Combat, v: &mut crate::ViewState) {
    let mut n = c.naval.take().unwrap();
    let id = n.raiders[2].id;
    n.raiders.retain(|r| r.slot == 2);
    n.raiders[0].stores = 0;
    n.raiders[0].aam = 0;
    n.raiders[0].next_shot = f64::INFINITY;
    for (i, t) in c.targets.iter_mut().enumerate() {
        t.alive = i == 2;
        t.health = 2.;
    }
    for s in &mut n.ships {
        s.standard = 0;
        s.essm = 0;
        s.gun = 0;
    }
    n.next_countermeasures = f64::INFINITY;
    n.rounds.clear();
    let p = carrier::ship_world(f, 1, DVec3::new(0., 25., 0.), 0.);
    n.round(
        TargetId::Ship(1),
        TargetId::Aircraft(id),
        Kind::Standard,
        p,
        carrier::rotation(0.) * DVec3::Y * 35. + carrier::velocity(0.),
    );
    let velocity = DVec3::Z * if mode == "sam-spent" { -480. } else { 480. };
    let range = if mode == "sam-spent" { 8000. } else { 16000. };
    for k in 0..12600 {
        let at = k as f64 / 120.;
        f.elapsed = at;
        n.clock = at;
        c.clock = at;
        let p = carrier::world(DVec3::new(0., 10000., -range) + velocity * at, at);
        let velocity = carrier::rotation(at) * velocity + carrier::velocity(at);
        let t = &mut c.targets[2];
        t.position = p;
        t.velocity = velocity;
        t.orientation = facing(velocity.normalize(), p.normalize());
        n.tracks = vec![Track {
            id,
            position: p,
            velocity,
            seen: at,
            hits: 5,
            role: Some(Role::Bomber),
        }];
        n.weapons(c, f, 1. / 120.);
        if !n.rounds[0].alive
            || (mode == "sam-terminal" && at >= 18.4167)
            || (mode == "sam-boost" && at >= 5.)
        {
            break;
        }
    }
    let m = &n.rounds[0];
    let reason = m.visual.gnc.reason;
    if mode == "sam-hit" {
        assert!(!m.alive && !c.targets[2].alive);
    }
    if mode == "sam-spent" {
        assert!(!m.alive && c.targets[2].alive);
    }
    println!(
        "SAM NATIVE {mode}: age={} live={} kill={} reason={reason} speed={}",
        m.age,
        m.alive,
        n.ship_kills,
        m.velocity.length()
    );
    n.report = format!(
        "SAM REVIEW / {mode} / LIVE {} / KILLS {} / {reason}",
        n.rounds.iter().filter(|m| m.alive).count(),
        n.ship_kills
    );
    n.report_until = f64::INFINITY;
    c.naval = Some(n);
    f.paused = true;
    v.restore_camera();
    v.index = 0;
    v.simple = true;
    v.help = false;
}
#[cfg(test)]
mod tests {
    use super::*;
    fn round(kind: Kind, speed: f64, age: f64) -> Round {
        let p = DVec3::Y * (RADIUS + 10000.);
        let v = DVec3::NEG_Z * speed;
        Round {
            sam: Default::default(),
            id: 1,
            source: TargetId::Ship(1),
            target: TargetId::Player,
            kind,
            position: p,
            previous: p,
            velocity: v,
            age,
            alive: true,
            support_lost: 0.,
            next_sensor: 0.,
            visual: render_round(p, v),
        }
    }
    #[test]
    fn burnout_turning_costs_energy_and_spent_or_missed_rounds_retire() {
        let mut straight = round(Kind::Standard, 800., 16.);
        let mut turn = straight.clone();
        let target = turn.position + DVec3::X * 4000.;
        for _ in 0..240 {
            straight.age += 1. / 240.;
            turn.age += 1. / 240.;
            advance(&mut straight, None, 1. / 240.);
            advance(&mut turn, Some((target, DVec3::ZERO)), 1. / 240.);
        }
        assert!(turn.velocity.length() < straight.velocity.length() - 1.);
        let mut spent = round(Kind::Essm, 120., 15.);
        advance(&mut spent, None, 1. / 120.);
        assert!(!spent.alive);
        assert_eq!(spent.visual.gnc.reason, "ENERGY DEPLETED");
        let mut missed = round(Kind::Standard, 700., 20.);
        missed.sam.best_range = 50.;
        for _ in 0..480 {
            missed.age += 1. / 120.;
            let t = missed.position + DVec3::Z * 3000.;
            advance(&mut missed, Some((t, DVec3::Z * 700.)), 1. / 120.);
            if !missed.alive {
                break;
            }
        }
        assert!(!missed.alive);
        assert_eq!(missed.visual.gnc.reason, "MISSED INTERCEPT");
        let mut expired = round(Kind::Standard, 700., 100.1);
        advance(&mut expired, None, 1. / 120.);
        assert!(!expired.alive);
    }
    fn engagement(kind: Kind, velocity: DVec3, range: f64, hz: usize, expect_hit: bool) -> f64 {
        let mut f = flight(true);
        let mut c = Combat::default();
        start(&mut c, &f, 1, 1);
        let mut n = c.naval.take().unwrap();
        let id = n.raiders[2].id;
        n.raiders.retain(|r| r.slot == 2);
        n.raiders[0].stores = 0;
        n.raiders[0].aam = 0;
        n.raiders[0].next_shot = f64::INFINITY;
        for (i, t) in c.targets.iter_mut().enumerate() {
            t.alive = i == 2;
            t.health = 2.;
        }
        for s in &mut n.ships {
            s.standard = 0;
            s.essm = 0;
            s.gun = 0;
        }
        n.next_countermeasures = f64::INFINITY;
        let mount = carrier::ship_world(&f, 1, DVec3::new(0., 25., 0.), 0.);
        assert!(n.round(
            TargetId::Ship(1),
            TargetId::Aircraft(id),
            kind,
            mount,
            carrier::rotation(0.) * DVec3::Y * 35. + carrier::velocity(0.)
        ));
        let mut peak = 0f64;
        for k in 0..hz * 105 {
            let t = k as f64 / hz as f64;
            f.elapsed = t;
            n.clock = t;
            let p = carrier::world(DVec3::new(0., 10000., -range) + velocity * t, t);
            let v = carrier::rotation(t) * velocity + carrier::velocity(t);
            c.targets[2].position = p;
            c.targets[2].velocity = v;
            n.tracks = vec![Track {
                id,
                position: p,
                velocity: v,
                seen: t,
                hits: 5,
                role: Some(Role::Bomber),
            }];
            n.weapons(&mut c, &mut f, 1. / hz as f64);
            peak = peak.max(n.rounds[0].velocity.length());
            if !n.rounds[0].alive {
                println!(
                    "SAM {kind:?} v={velocity:?} hz={hz} end={t} peak={peak} killed={} reason={} miss={}",
                    !c.targets[2].alive,
                    n.rounds[0].visual.gnc.reason,
                    n.rounds[0].position.distance(p)
                );
                assert_eq!(
                    !c.targets[2].alive,
                    expect_hit,
                    "bomber intercept reason={} miss={}",
                    n.rounds[0].visual.gnc.reason,
                    n.rounds[0].position.distance(p)
                );
                assert_eq!(n.ship_kills, if expect_hit { 1 } else { 0 });
                return t;
            }
        }
        panic!("unbounded SAM flight");
    }
    #[test]
    fn standard_intercepts_fast_approaching_crossing_and_receding_bombers() {
        engagement(Kind::Standard, DVec3::Z * 480., 16000., 120, true);
        engagement(Kind::Standard, DVec3::X * 480., 12000., 120, true);
        engagement(Kind::Standard, DVec3::NEG_Z * 300., 8000., 120, true);
        assert!(engagement(Kind::Standard, DVec3::NEG_Z * 480., 8000., 120, false) < 100.);
    }
    #[test]
    fn essm_intercept_converges_across_frame_rates() {
        let reference = engagement(Kind::Essm, DVec3::Z * 450., 10000., 240, true);
        for hz in [60, 120] {
            let t = engagement(Kind::Essm, DVec3::Z * 450., 10000., hz, true);
            assert!((t - reference).abs() < 0.08);
        }
    }
    #[test]
    fn intercept_time_and_command_follow_collision_geometry() {
        let r = DVec3::NEG_Z * 10000.;
        let v = DVec3::Z * 500.;
        let t = lead_time(r, v, 1000.).unwrap();
        assert!((t - 10000. / 1500.).abs() < 1e-9);
        assert!(lead_time(r, -v, 400.).is_none());
        let a = command(DVec3::ZERO, DVec3::NEG_Z * 900., r, DVec3::X * 300., 1000.);
        assert!(a.x > 0.);
        assert!(a.z.abs() < 1e-9);
    }
}
