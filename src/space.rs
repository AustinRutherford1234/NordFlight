//! Real-scale, game-tuned direct-return lunar mission. State stays in the same
//! Earth-rotating f64 frame across patched-conic transitions; no teleporting.
use crate::physics::*;
use bevy::math::{DQuat, DVec3};
#[path = "space_navigation.rs"]
pub mod navigation;
pub const MOON_RADIUS: f64 = 1_737_400.;
pub const MOON_MU: f64 = 4.904_869_5e12;
pub const MOON_DISTANCE: f64 = 384_400_000.;
pub const MOON_PERIOD: f64 = 27.321661 * 86400.;
pub const MOON_SOI: f64 = 66_183_000.;
pub const G0: f64 = 9.80665;
pub const SAS_MODES: [&str; 8] = [
    "RATE HOLD",
    "PROGRADE",
    "RETROGRADE",
    "RADIAL OUT",
    "NORMAL +",
    "MANEUVER",
    "RADIAL IN",
    "NORMAL −",
];
pub fn attitude_direction(f: &Flight) -> Option<DVec3> {
    let (r, v) = relative(f);
    let direction = match f.sas_mode {
        1 => v,
        2 => -v,
        3 => r,
        4 => r.cross(v),
        5 => f.space.burn_direction,
        6 => -r,
        7 => -r.cross(v),
        _ => return None,
    }
    .normalize_or_zero();
    (direction.length_squared() > 0.5).then_some(direction)
}
pub fn sas_label(f: &Flight) -> &'static str {
    SAS_MODES[f.sas_mode.min(7) as usize]
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Body {
    #[default]
    Earth,
    Moon,
}
impl Body {
    pub fn label(self) -> &'static str {
        if self == Self::Earth { "EARTH" } else { "MOON" }
    }
    pub fn mu(self) -> f64 {
        if self == Self::Earth { MU } else { MOON_MU }
    }
    pub fn radius(self) -> f64 {
        if self == Self::Earth {
            RADIUS
        } else {
            MOON_RADIUS
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Stage {
    #[default]
    Transfer,
    Descent,
    Return,
    Capsule,
}
impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Transfer => "TRANSFER",
            Self::Descent => "LUNAR DESCENT",
            Self::Return => "ASCENT / EARTH RETURN",
            Self::Capsule => "REENTRY CAPSULE",
        }
    }
    pub fn engine(self) -> (f64, f64) {
        match self {
            Self::Transfer => (1_000_000., 421.),
            Self::Descent => (186_800., 311.),
            Self::Return => (91_000., 314.),
            Self::Capsule => (0., 0.),
        }
    }
    pub fn capacity(self) -> f64 {
        match self {
            Self::Transfer => 170000.,
            Self::Descent => 34000.,
            Self::Return => 19000.,
            Self::Capsule => 0.,
        }
    }
}
#[derive(Clone)]
pub struct Debris {
    pub stage: Stage,
    pub position: DVec3,
    pub velocity: DVec3,
    pub orientation: DQuat,
    pub grounded: bool,
    pub epoch: f64,
    pub body: Body,
}
#[derive(Clone)]
pub struct State {
    pub body: Body,
    pub stage: Stage,
    pub descent_fuel: f64,
    pub return_fuel: f64,
    pub rcs_fuel: f64,
    pub heat: f64,
    pub heat_load: f64,
    pub shield: f64,
    pub chute_armed: bool,
    pub chute: f64,
    pub legs: [f64; 4],
    pub debris: Vec<Debris>,
    pub visited_moon: bool,
    pub burn_direction: DVec3,
    pub burn_stop: Option<f64>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            body: Body::Earth,
            stage: Stage::Transfer,
            descent_fuel: 34000.,
            return_fuel: 19000.,
            rcs_fuel: 400.,
            heat: 0.,
            heat_load: 0.,
            shield: 1.,
            chute_armed: false,
            chute: 0.,
            legs: [0.; 4],
            debris: Vec::new(),
            visited_moon: false,
            burn_direction: DVec3::ZERO,
            burn_stop: None,
        }
    }
}
pub fn earth_rotation(t: f64) -> DQuat {
    let w = rotation();
    DQuat::from_axis_angle(w.normalize(), w.length() * t)
}
pub fn moon_inertial(t: f64) -> (DVec3, DVec3, DVec3, DQuat) {
    // Circular mean-distance ephemeris, not a date-calibrated lunar ephemeris.
    let tilt = DQuat::from_rotation_y(5.145_f64.to_radians());
    let axis = tilt * -DVec3::Z;
    let w = axis * (std::f64::consts::TAU / MOON_PERIOD);
    let q = DQuat::from_axis_angle(axis, 2.5 + w.length() * t) * tilt;
    let p = q * DVec3::Y * MOON_DISTANCE;
    (p, w.cross(p), w.cross(w.cross(p)), q)
}
pub fn moon(t: f64) -> (DVec3, DVec3, DQuat) {
    let (p, v, _, q) = moon_inertial(t);
    let rot = earth_rotation(t).conjugate();
    (rot * p, rot * v, rot * q)
}
pub fn center(body: Body, t: f64) -> DVec3 {
    if body == Body::Earth {
        DVec3::ZERO
    } else {
        moon(t).0
    }
}
pub fn primary(p: DVec3, t: f64) -> Body {
    if p.distance(moon(t).0) < MOON_SOI {
        Body::Moon
    } else {
        Body::Earth
    }
}
pub fn relative(f: &Flight) -> (DVec3, DVec3) {
    let (center, velocity) = if f.space.body == Body::Moon {
        let (p, v, _) = moon(f.elapsed);
        (p, v)
    } else {
        (DVec3::ZERO, DVec3::ZERO)
    };
    (
        f.position - center,
        f.velocity + rotation().cross(f.position) - velocity,
    )
}
pub fn ground_velocity(p: DVec3, t: f64, body: Body) -> DVec3 {
    if body == Body::Earth {
        return DVec3::ZERO;
    }
    let (c, v, q) = moon(t);
    let axis = q * -DVec3::Z;
    v + axis.cross(p - c) * (std::f64::consts::TAU / MOON_PERIOD) - rotation().cross(p)
}
pub fn mass(f: &Flight) -> f64 {
    let s = &f.space;
    let return_dry = 8000.;
    let base = match s.stage {
        Stage::Transfer => return_dry + s.return_fuel + 5000. + s.descent_fuel + 7000.,
        Stage::Descent => return_dry + s.return_fuel + 5000.,
        Stage::Return => return_dry,
        Stage::Capsule => 5500.,
    };
    base + f.fuel + s.rcs_fuel
}
pub fn delta_v(f: &Flight) -> f64 {
    let (_, isp) = f.space.stage.engine();
    isp * G0 * (f.mass() / (f.mass() - f.fuel).max(1.)).ln()
}
pub fn future_delta_v(f: &Flight) -> (f64, f64) {
    let s = &f.space;
    let ret = 8000. + s.rcs_fuel;
    let rdv = 314. * G0 * ((ret + s.return_fuel) / ret).ln();
    let ddry = ret + s.return_fuel + 5000.;
    let ddv = 311. * G0 * ((ddry + s.descent_fuel) / ddry).ln();
    (
        if s.stage == Stage::Transfer { ddv } else { 0. },
        if matches!(s.stage, Stage::Transfer | Stage::Descent) {
            rdv
        } else {
            0.
        },
    )
}
pub fn stage(f: &mut Flight) -> bool {
    if f.vehicle != Vehicle::Spacecraft || f.crashed || f.space.stage == Stage::Capsule {
        return false;
    }
    if f.throttle > 0.001 {
        f.status = "CUT THROTTLE BEFORE STAGING".into();
        return false;
    }
    if f.space.stage == Stage::Descent && !f.landed && f.agl() < 30. {
        f.status = "STAGE AFTER TOUCHDOWN OR ABOVE 30 M".into();
        return false;
    }
    let old = f.space.stage;
    f.space.debris.push(Debris {
        stage: old,
        position: f.position,
        velocity: f.velocity + f.forward() * -0.5,
        orientation: f.orientation,
        grounded: f.landed,
        epoch: f.elapsed,
        body: f.space.body,
    });
    if f.space.debris.len() > 3 {
        f.space.debris.remove(0);
    }
    f.space.stage = match old {
        Stage::Transfer => Stage::Descent,
        Stage::Descent => Stage::Return,
        _ => Stage::Capsule,
    };
    f.fuel = match f.space.stage {
        Stage::Descent => std::mem::take(&mut f.space.descent_fuel),
        Stage::Return => std::mem::take(&mut f.space.return_fuel),
        _ => 0.,
    };
    f.spool = 0.;
    f.thrust = 0.;
    f.gear = f.space.stage == Stage::Descent && f.agl() < 1000.;
    f.gear_extension = if f.gear { 1. } else { 0. };
    if f.landed {
        f.position += f.forward() * 0.7;
        f.landed = false;
    }
    f.status = format!(
        "STAGED / {} / {:.0} m/s REMAINING",
        f.space.stage.label(),
        delta_v(f)
    );
    true
}
pub fn debris_pose(d: &Debris, time: f64) -> Option<(DVec3, DQuat)> {
    if d.grounded {
        let (c0, _, q0) = moon(d.epoch);
        let (c, _, q) = moon(time);
        return Some((
            c + q * (q0.conjugate() * (d.position - c0)),
            q * q0.conjugate() * d.orientation,
        ));
    }
    let dt = time - d.epoch;
    if dt > 86400. || dt < 0. {
        return None;
    }
    let q0 = earth_rotation(d.epoch);
    let q1 = earth_rotation(time).conjugate();
    let (c0, v0) = if d.body == Body::Moon {
        let (p, v, _, _) = moon_inertial(d.epoch);
        (p, v)
    } else {
        (DVec3::ZERO, DVec3::ZERO)
    };
    let (c1, _) = if d.body == Body::Moon {
        let (p, v, _, _) = moon_inertial(time);
        (p, v)
    } else {
        (DVec3::ZERO, DVec3::ZERO)
    };
    let (p, _) = propagate(
        q0 * d.position - c0,
        q0 * (d.velocity + rotation().cross(d.position)) - v0,
        d.body.mu(),
        dt,
    )?;
    if p.length() < d.body.radius() {
        return None;
    }
    Some((q1 * (p + c1), q1 * q0 * d.orientation))
}
pub fn scenario(id: u8) -> Flight {
    let mut f = Flight::spacecraft();
    if id == 3 {
        return f;
    }
    f.space.body = Body::Moon;
    f.space.stage = Stage::Descent;
    f.space.descent_fuel = 0.;
    f.fuel = 34000.;
    f.gear = id != 9;
    f.gear_extension = if f.gear { 1. } else { 0. };
    let (c, _, q) = moon(0.);
    let site = if id == 9 { DVec3::X } else { landing_site() };
    let radial = q * site;
    if id == 9 {
        f.position = c + radial * (MOON_RADIUS + 100000.);
        let tangent = (q * DVec3::Y).normalize();
        let v = moon(0.).1 + tangent * (MOON_MU / (MOON_RADIUS + 100000.)).sqrt();
        f.velocity = v - rotation().cross(f.position);
        f.orientation = DQuat::from_rotation_arc(-DVec3::Z, tangent);
    } else {
        f.position = c + radial * (MOON_RADIUS + height(site) + 1500.);
        f.velocity = ground_velocity(f.position, 0., Body::Moon) - radial * 18.;
        f.orientation = DQuat::from_rotation_arc(-DVec3::Z, radial);
        f.sas_mode = 3;
        f.assist = true;
    }
    f.density = 0.;
    f.pressure = 0.;
    f.airspeed = relative(&f).1.length();
    f.mach = 0.;
    f.status = "LUNAR FLIGHT / SPACE STAGE / G LEGS / M MAP".into();
    f
}
fn hash(x: i64, y: i64) -> f64 {
    let mut v =
        (x as u64).wrapping_mul(0x9e3779b97f4a7c15) ^ (y as u64).wrapping_mul(0xbf58476d1ce4e5b9);
    v = (v ^ (v >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    ((v ^ (v >> 27)) >> 11) as f64 / (1u64 << 53) as f64
}
pub fn landing_site() -> DVec3 {
    static SITE: std::sync::OnceLock<DVec3> = std::sync::OnceLock::new();
    *SITE.get_or_init(|| {
        let mut best = (f64::NEG_INFINITY, DVec3::X);
        for ring in 0i32..=24 {
            for y in -ring..=ring {
                for z in -ring..=ring {
                    if y.abs() != ring && z.abs() != ring {
                        continue;
                    }
                    let n = DVec3::new(MOON_RADIUS, y as f64 * 150., z as f64 * 150.).normalize();
                    let score = normal(n).dot(n);
                    if score > best.0 {
                        best = (score, n);
                    }
                    if score > 0.9999 {
                        return n;
                    }
                }
            }
        }
        best.1
    })
}
fn destroy(f: &mut Flight, cause: &str, normal: DVec3) {
    let relative = f.velocity - ground_velocity(f.position, f.elapsed, f.space.body);
    let energy = 0.5 * f.mass() * relative.length_squared();
    crate::collision::destroy(f, cause, normal);
    f.impact_energy = energy;
    f.status = format!("{} / IMPACT {:.0} kJ / R TO RESET", cause, energy / 1000.);
}
/// Procedural, shared visual/contact terrain. No claim of surveyed lunar relief.
pub fn height(direction: DVec3) -> f64 {
    let n = direction.normalize_or(DVec3::X);
    let x = n.z.atan2(n.x) * MOON_RADIUS;
    let y = n.y.clamp(-1., 1.).asin() * MOON_RADIUS;
    let mut h = 180. * (n.x * 8. + n.z * 3.).sin() * (n.y * 9. - n.z * 5.).cos();
    for spacing in [80000., 12000., 1500., 180.] {
        let cells = (std::f64::consts::TAU * MOON_RADIUS / spacing).round() as i64;
        let spacing = std::f64::consts::TAU * MOON_RADIUS / cells as f64;
        let noise = |a: i64, b: i64| hash(a.rem_euclid(cells), b);
        let ix = (x / spacing).floor() as i64;
        let iy = (y / spacing).floor() as i64;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let a = ix + dx;
                let b = iy + dy;
                let seed = noise(a, b);
                if seed < 0.4 {
                    continue;
                }
                let cx = (a as f64 + noise(a + 73, b - 18)) * spacing;
                let cy = (b as f64 + noise(a - 19, b + 52)) * spacing;
                let radius = spacing * (0.12 + seed * 0.27);
                let r = (x - cx).hypot(y - cy) / radius;
                if r < 1.5 {
                    h += radius
                        * ((1. - n.y.abs()) / 0.02).clamp(0., 1.)
                        * (-0.15 * (1. - r * r).max(0.).powi(2)
                            + 0.055 * (-((r - 1.) / 0.16).powi(2)).exp());
                }
            }
        }
    }
    h
}
pub fn terrain_at(p: DVec3, t: f64) -> (f64, DVec3) {
    let (c, _, q) = moon(t);
    let n = q.conjugate() * (p - c).normalize();
    (height(n), n)
}
pub fn leg_point(f: &Flight, i: usize) -> DVec3 {
    let mut point = leg_anchor(f, i);
    point.z -= f.space.legs[i];
    point
}
fn leg_anchor(f: &Flight, i: usize) -> DVec3 {
    let (x, y) = [(-6.5, -6.5), (-6.5, 6.5), (6.5, -6.5), (6.5, 6.5)][i];
    let extension = f.gear_extension.clamp(0., 1.);
    let extension = extension * extension * (3. - 2. * extension);
    DVec3::new(
        x * (0.37 + 0.63 * extension),
        y * (0.37 + 0.63 * extension),
        6.4 + 1.8 * extension,
    )
}
/// Axisymmetric stage envelopes for low-altitude body clearance. The four feet
/// retain their separate terrain-normal spring contacts.
pub fn body_sections(stage: Stage) -> &'static [(f64, f64)] {
    const CAPSULE: &[(f64, f64)] = &[(-4.15, 0.35), (-2., 1.28), (1., 2.1), (1.72, 1.9)];
    const RETURN: &[(f64, f64)] = &[
        (-4.15, 0.35),
        (-2., 1.28),
        (1., 2.1),
        (1.8, 1.9),
        (4.1, 1.9),
        (6.1, 0.95),
    ];
    const DESCENT: &[(f64, f64)] = &[
        (-4.15, 0.35),
        (-2., 1.28),
        (1., 2.1),
        (4.4, 3.7),
        (7.4, 3.7),
        (7.7, 3.6),
    ];
    const TRANSFER: &[(f64, f64)] = &[
        (-4.15, 0.35),
        (-2., 1.28),
        (1., 2.1),
        (4.4, 3.7),
        (7.4, 3.7),
        (8.7, 3.2),
        (17.5, 3.2),
        (20.6, 1.65),
    ];
    match stage {
        Stage::Transfer => TRANSFER,
        Stage::Descent => DESCENT,
        Stage::Return => RETURN,
        Stage::Capsule => CAPSULE,
    }
}
pub fn body_offset(f: &Flight) -> f64 {
    let c = (f.orientation * DVec3::Z).dot(f.up()).clamp(-1., 1.);
    let s = (1. - c * c).sqrt();
    body_sections(f.space.stage)
        .iter()
        .map(|(z, r)| z * c - r * s)
        .fold(f64::INFINITY, f64::min)
}
pub fn normal(n: DVec3) -> DVec3 {
    let n = n.normalize();
    let x = n.cross(DVec3::Y).normalize_or(DVec3::Z);
    let y = n.cross(x);
    let sample = |d: DVec3| {
        let p = (n + d / MOON_RADIUS).normalize();
        p * (MOON_RADIUS + height(p))
    };
    let a = sample(x * 1.) - sample(-x * 1.);
    let b = sample(y * 1.) - sample(-y * 1.);
    a.cross(b).normalize_or(n)
}
pub struct Aero {
    pub force: DVec3,
    pub torque: DVec3,
    pub cd: f64,
    pub cl: f64,
    pub heat: f64,
}
pub fn aero(f: &Flight, air: DVec3, rho: f64) -> Aero {
    let speed = air.length();
    let b = f.orientation.conjugate() * air;
    let flow = b.normalize_or_zero();
    let aft = flow.z.clamp(-1., 1.);
    let capsule = f.space.stage == Stage::Capsule;
    let area = if capsule {
        12.6
    } else {
        if f.space.stage == Stage::Transfer {
            34.
        } else {
            24.
        }
    };
    let q = 0.5 * rho * speed * speed;
    let cd = if capsule {
        0.92 + 0.43 * aft.max(0.).powi(2) + 0.12 * (-((f.mach - 1.) / 0.4).powi(2)).exp()
    } else {
        0.75 + 0.8 * (1. - aft * aft)
    };
    let cl = if capsule {
        0.28 * (2. * aft.acos()).sin()
    } else {
        0.08 * (2. * aft.acos()).sin()
    };
    let lift = (DVec3::Z - flow * aft).normalize_or_zero();
    let force = (-flow * cd + lift * cl) * q * area - flow * (q * 750. * 1.5 * f.space.chute);
    let arm = if capsule { -0.65 } else { -1.8 };
    let torque = (DVec3::Z * arm).cross(force) - f.rates * (q * area * 2.4 / speed.max(10.));
    let heat = 1.83e-4 * (rho / 1.8).sqrt() * speed.powi(3);
    Aero {
        force: f.orientation * force,
        torque,
        cd,
        cl,
        heat,
    }
}
// Universal-variable Kepler drift, used for coast warp and trajectory prediction.
fn stumpff(z: f64) -> (f64, f64) {
    if z.abs() < 1e-5 {
        (
            0.5 - z / 24. + z * z / 720.,
            1. / 6. - z / 120. + z * z / 5040.,
        )
    } else if z > 0. {
        let a = z.sqrt();
        ((1. - a.cos()) / z, (a - a.sin()) / a.powi(3))
    } else {
        let a = (-z).sqrt();
        ((a.cosh() - 1.) / (-z), (a.sinh() - a) / a.powi(3))
    }
}
pub fn propagate(r: DVec3, v: DVec3, mu: f64, dt: f64) -> Option<(DVec3, DVec3)> {
    let r0 = r.length();
    if r0 < 1. || !dt.is_finite() {
        return None;
    }
    let sm = mu.sqrt();
    let vr = r.dot(v) / r0;
    let alpha = 2. / r0 - v.length_squared() / mu;
    let mut x = if alpha.abs() > 1e-9 {
        sm * alpha.abs() * dt
    } else {
        sm * dt / r0
    };
    let mut converged = false;
    for _ in 0..80 {
        let z = alpha * x * x;
        let (c, s) = stumpff(z);
        let fun = r0 * vr / sm * x * x * c + (1. - alpha * r0) * x.powi(3) * s + r0 * x - sm * dt;
        let derivative = r0 * vr / sm * x * (1. - z * s) + (1. - alpha * r0) * x * x * c + r0;
        let step = fun / derivative;
        x -= step;
        if !x.is_finite() {
            return None;
        }
        if step.abs() < 1e-7 {
            converged = true;
            break;
        }
    }
    if !converged {
        return None;
    }
    let (c, s) = stumpff(alpha * x * x);
    let ff = 1. - x * x / r0 * c;
    let g = dt - x.powi(3) / sm * s;
    let p = r * ff + v * g;
    let rn = p.length();
    let vv = r * (sm / (rn * r0) * (alpha * x.powi(3) * s - x)) + v * (1. - x * x / rn * c);
    (p.is_finite() && vv.is_finite()).then_some((p, vv))
}
pub fn coast(f: &mut Flight, e: &mut Environment, duration: f64) -> f64 {
    let mut elapsed = 0.;
    while elapsed < duration - 1e-9 {
        f.space.body = primary(f.position, f.elapsed);
        let body = f.space.body;
        let limit = if body == Body::Earth { 150000. } else { 20000. };
        if f.agl() < limit {
            break;
        }
        let t = f.elapsed;
        let q = earth_rotation(t);
        let (p0, v0) = if body == Body::Moon {
            let (p, v, _, _) = moon_inertial(t);
            (p, v)
        } else {
            (DVec3::ZERO, DVec3::ZERO)
        };
        let r = q * f.position - p0;
        let v = q * (f.velocity + rotation().cross(f.position)) - v0;
        let lunar_range = f.position.distance(moon(t).0);
        let boundary = (lunar_range - MOON_SOI).abs() / v.length().max(1.);
        let mut dt = (duration - elapsed)
            .min(60.)
            .min(boundary.mul_add(0.15, 0.01).max(0.05));
        let (p, vv) = loop {
            if let Some(state) = propagate(r, v, body.mu(), dt) {
                break state;
            }
            dt *= 0.5;
            if dt < 1e-5 {
                return elapsed;
            }
        };
        let center1 = if body == Body::Moon {
            moon_inertial(t + dt).0
        } else {
            DVec3::ZERO
        };
        let vel1 = if body == Body::Moon {
            moon_inertial(t + dt).1
        } else {
            DVec3::ZERO
        };
        let q1 = earth_rotation(t + dt).conjugate();
        let next = q1 * (p + center1);
        let next_body = primary(next, t + dt);
        let alt = (next - center(next_body, t + dt)).length() - next_body.radius();
        if alt < limit + 2000. && dt > 0.1 {
            // Stop before atmospheric/terrain entry instead of skipping it.
            let safe = (f.agl() - limit - 2000.) / v.length().max(1.);
            if safe < 0.1 {
                break;
            }
            let actual = coast(f, e, dt.min(safe * 0.5));
            elapsed += actual;
            if actual < 0.1 {
                break;
            }
            continue;
        }
        f.position = next;
        f.velocity = q1 * (vv + vel1) - rotation().cross(next);
        f.orientation = (DQuat::from_scaled_axis(-rotation() * dt) * f.orientation).normalize();
        f.elapsed += dt;
        e.advance(dt);
        elapsed += dt;
        if next_body != body {
            f.space.body = next_body;
            f.status = format!("ENTERED {} SPHERE OF INFLUENCE", next_body.label());
            break;
        }
    }
    f.thrust = 0.;
    f.spool = 0.;
    f.rcs_torque = DVec3::ZERO;
    let (t, p, rho, a) = if f.space.body == Body::Moon {
        (3., 0., 0., 1.)
    } else {
        atmosphere(f.altitude())
    };
    f.temperature = t;
    f.pressure = p;
    f.density = rho;
    f.airspeed = if f.space.body == Body::Moon {
        relative(f).1.length()
    } else {
        f.velocity.length()
    };
    f.mach = if rho > 0. { f.airspeed / a } else { 0. };
    elapsed
}
pub fn simulate(f: &mut Flight, e: &Environment, dt: f64) {
    let old_t = f.elapsed;
    f.space.body = primary(f.position, old_t);
    let body = f.space.body;
    f.gear_extension +=
        ((if f.gear { 1. } else { 0. }) - f.gear_extension).clamp(-dt / 3., dt / 3.);
    let r = f.position - center(body, old_t);
    let up = r.normalize();
    let (t, p, rho, a) = if body == Body::Moon {
        (3., 0., 0., 1.)
    } else {
        atmosphere(r.length() - RADIUS)
    };
    f.temperature = t;
    f.pressure = p;
    f.density = rho;
    let wind = if body == Body::Earth {
        e.wind(f.position)
    } else {
        DVec3::ZERO
    };
    let air = f.velocity - wind;
    f.wind = wind;
    f.airspeed = if body == Body::Moon {
        relative(f).1.length()
    } else {
        air.length()
    };
    f.mach = if rho > 0. { f.airspeed / a } else { 0. };
    let b = f.orientation.conjugate() * air;
    f.alpha = b.y.atan2(-b.z);
    f.beta = (b.x / b.length().max(1.)).clamp(-1., 1.).asin();
    if f.space.chute_armed
        && f.space.stage == Stage::Capsule
        && body == Body::Earth
        && f.altitude() < 8000.
        && air.length() < 350.
    {
        f.space.chute = (f.space.chute + dt / 5.).min(1.);
    }
    f.spool += (f.throttle - f.spool) * (1. - (-dt / 0.35).exp());
    let (max_thrust, isp) = f.space.stage.engine();
    let flow = max_thrust * f.spool / (isp * G0).max(1.);
    let fraction = if !f.engine || f.crashed {
        0.
    } else if f.infinite_fuel {
        1.
    } else {
        (f.fuel / (flow * dt).max(1e-12)).clamp(0., 1.)
    };
    f.thrust = max_thrust * f.spool * fraction;
    if !f.infinite_fuel {
        f.fuel = (f.fuel - flow * dt * fraction).max(0.);
    }
    let aero = aero(f, air, rho);
    f.aero_coeff.drag = aero.cd;
    f.aero_coeff.lift = aero.cl;
    f.space.heat = aero.heat;
    f.space.heat_load += aero.heat * dt;
    if body == Body::Earth && rho > 1e-8 {
        let exposed = (1. - (f.orientation.conjugate() * air.normalize_or_zero()).z).clamp(0., 1.);
        let protection = if f.space.stage == Stage::Capsule {
            1.
        } else {
            0.1
        };
        f.space.shield = (f.space.shield - aero.heat * dt / 1.6e9 / protection).max(0.);
        if (f.space.shield <= 0. || (exposed > 0.7 && aero.heat > 2e6)) && aero.heat > 1e5 {
            destroy(f, "REENTRY THERMAL FAILURE", up);
            return;
        }
    }
    let desired = if f.assist && f.sas_mode > 0 && f.controls.length_squared() < 0.001 {
        let direction = attitude_direction(f).unwrap_or(DVec3::ZERO);
        if direction.length_squared() < 0.5 {
            DVec3::ZERO
        } else {
            let error = DQuat::from_rotation_arc(-DVec3::Z, f.orientation.conjugate() * direction)
                .to_scaled_axis();
            // A heavy transfer stack needs to start braking well before alignment.
            let length = match f.space.stage {
                Stage::Transfer => 16.,
                Stage::Descent => 8.,
                Stage::Return => 5.,
                Stage::Capsule => 2.5,
            };
            let moment = DVec3::new(length * length / 12., length * length / 12., 2.) * f.mass();
            DVec3::from_array(std::array::from_fn(|i| {
                error[i].signum() * (2. * 18000. / moment[i] * error[i].abs()).sqrt().min(0.2) * 0.7
            }))
        }
    } else {
        DVec3::new(f.controls.x, f.controls.y, -f.controls.z) * 0.35
    };
    let m = f.mass();
    let length = match f.space.stage {
        Stage::Transfer => 16.,
        Stage::Descent => 8.,
        Stage::Return => 5.,
        Stage::Capsule => 2.5,
    };
    let inertia = DVec3::new(length * length / 12., length * length / 12., 2.) * m;
    let requested = if f.assist {
        (desired - f.rates) * inertia * 1.6
    } else {
        DVec3::new(f.controls.x, f.controls.y, -f.controls.z) * 15000.
    };
    let available = f.space.rcs_fuel > 0. || f.infinite_fuel;
    let torque = if available {
        requested.clamp(DVec3::splat(-18000.), DVec3::splat(18000.))
    } else {
        DVec3::ZERO
    };
    f.rcs_torque = torque;
    if !f.infinite_fuel {
        f.space.rcs_fuel =
            (f.space.rcs_fuel - torque.abs().element_sum() / (3. * 220. * G0) * dt).max(0.);
    }
    let mut forces = f.forward() * f.thrust + aero.force;
    let mut moments = torque + aero.torque;
    let mut contacts = 0;
    let mut max_pen: f64 = 0.;
    let mut contact_normal = DVec3::ZERO;
    f.space.legs = [0.; 4];
    if body == Body::Moon && f.space.stage == Stage::Descent && f.gear && f.gear_extension > 0.95 {
        let (c, _, q) = moon(old_t);
        let ground = ground_velocity(f.position, old_t, body);
        for i in 0..4 {
            let arm = leg_anchor(f, i);
            let point = f.position + f.orientation * arm;
            let nbody = q.conjugate() * (point - c).normalize();
            let n = q * normal(nbody);
            let pen = MOON_RADIUS + height(nbody) + 0.15 - (point - c).length();
            if pen > 0. {
                contacts += 1;
                max_pen = max_pen.max(pen);
                contact_normal += n;
                let vel = f.velocity - ground + f.orientation * f.rates.cross(arm);
                let speed = vel.dot(n);
                let tangent = vel - n * speed;
                if speed < -5. || f.forward().dot(n) < 0.65 || pen > 1.2 {
                    destroy(f, "LUNAR LANDING / HARD CONTACT", n);
                    return;
                }
                let k = m * 1.62 / (4. * 0.22);
                let damping = 1.6 * (k * m / 4.).sqrt();
                let load = (k * pen - damping * speed).clamp(0., m * 1.62 * 2.);
                let friction = -tangent.normalize_or_zero()
                    * (load * 0.7).min(tangent.length() * m / (4. * dt));
                let force = n * load + friction;
                forces += force;
                moments += arm.cross(f.orientation.conjugate() * force);
                f.space.legs[i] = pen.min(0.6);
            }
        }
    }
    let gravity = -r * body.mu() / r.length().powi(3);
    let origin_acc = if body == Body::Moon {
        earth_rotation(old_t).conjugate() * moon_inertial(old_t).2
    } else {
        DVec3::ZERO
    };
    let w = rotation();
    let acc =
        gravity + origin_acc - 2. * w.cross(f.velocity) - w.cross(w.cross(f.position)) + forces / m;
    f.rates += (moments - f.rates.cross(inertia * f.rates)) / inertia * dt;
    f.orientation =
        (DQuat::from_scaled_axis(-w * dt) * f.orientation * DQuat::from_scaled_axis(f.rates * dt))
            .normalize();
    f.velocity += acc * dt;
    f.position += f.velocity * dt;
    f.elapsed += dt;
    f.g_load = forces.length() / (m * G0);
    f.space.body = primary(f.position, f.elapsed);
    let surface_velocity = ground_velocity(f.position, f.elapsed, f.space.body);
    let relspeed = f.velocity - surface_velocity;
    f.touchdown_sink = relspeed.dot(f.up());
    f.landed = contacts >= 3
        && relspeed.length() < 0.8
        && f.rates.length() < 0.1
        && f.thrust < m * 1.62 * 0.8;
    if f.landed && f.thrust < m * 1.62 * 0.8 {
        f.velocity = surface_velocity;
        f.rates *= (-dt * 8.).exp();
        f.space.visited_moon = true;
        // Spring equilibrium correction prevents sub-millimetre accumulated sink.
        if max_pen > 0.35 {
            f.position += contact_normal.normalize_or(up) * (max_pen - 0.35);
        }
    }
    let hull_offset = body_offset(f);
    if f.agl() + hull_offset < 0. {
        if body == Body::Earth && f.space.stage == Stage::Capsule && relspeed.length() < 13. {
            f.landed = true;
            f.velocity = DVec3::ZERO;
            f.throttle = 0.;
            f.engine = false;
            f.position = f.up()
                * (RADIUS + terrain_height(f.position.x, f.position.z).max(0.) - hull_offset);
            f.status = "CAPSULE LANDED / RECOVERY COMPLETE".into();
            return;
        } else {
            destroy(f, "SPACECRAFT SURFACE IMPACT", f.up());
            return;
        }
    }
    f.status = if f.landed {
        "LUNAR TOUCHDOWN / CUT THROTTLE / SPACE TO LEAVE DESCENT STAGE".into()
    } else {
        format!(
            "{} / {} / Δv {:.0} m/s / SPACE STAGE",
            f.space.body.label(),
            f.space.stage.label(),
            delta_v(f)
        )
    };
}
pub fn wreck(f: &mut Flight, dt: f64) {
    let body = primary(f.position, f.elapsed);
    f.space.body = body;
    let rel = f.position - center(body, f.elapsed);
    let w = rotation();
    let origin = if body == Body::Moon {
        earth_rotation(f.elapsed).conjugate() * moon_inertial(f.elapsed).2
    } else {
        DVec3::ZERO
    };
    let acc = -rel * body.mu() / rel.length().powi(3) + origin
        - 2. * w.cross(f.velocity)
        - w.cross(w.cross(f.position));
    f.velocity += acc * dt;
    f.position += f.velocity * dt;
    f.elapsed += dt;
    f.wreck_age += dt;
    f.orientation =
        (DQuat::from_scaled_axis(-w * dt) * f.orientation * DQuat::from_scaled_axis(f.rates * dt))
            .normalize();
    if f.agl() < 2.1 {
        let h = if body == Body::Moon {
            terrain_at(f.position, f.elapsed).0
        } else {
            terrain_height(f.position.x, f.position.z).max(0.)
        };
        f.position = center(body, f.elapsed) + f.up() * (body.radius() + h + 2.1);
        f.velocity = ground_velocity(f.position, f.elapsed, body);
        f.rates *= (-dt * 4.).exp();
    }
    f.thrust = 0.;
    f.rcs_torque = DVec3::ZERO;
}
pub fn diagnostic_landing() -> Flight {
    let mut f = scenario(10);
    let mut e = Environment::default();
    for _ in 0..72000 {
        let target = -((f.agl() - 8.5).max(0.) * 0.8).sqrt().clamp(0.2, 20.);
        let g = MOON_MU / (f.altitude() + MOON_RADIUS).powi(2);
        f.throttle = if f.space.legs.iter().filter(|p| **p > 0.).count() >= 2 {
            0.
        } else {
            (f.mass() * (g + 1.8 * (target - f.vertical_speed())) / f.space.stage.engine().0)
                .clamp(0., 1.)
        };
        e.advance(1. / 120.);
        crate::physics::simulate(&mut f, &e, 1. / 120.);
        if f.crashed || (f.landed && f.elapsed > 5.) {
            break;
        }
    }
    f
}
pub fn diagnostic_recovery() -> Flight {
    let mut f = Flight::spacecraft();
    f.space.stage = Stage::Capsule;
    f.fuel = 0.;
    f.space.chute_armed = true;
    f.assist = false;
    f.position = surface_point(0., 0., 5000.);
    f.velocity = -f.up() * 100.;
    f.orientation = DQuat::from_rotation_arc(DVec3::Z, -f.up());
    let mut e = Environment::default();
    e.wind_enabled = false;
    for _ in 0..120000 {
        e.advance(1. / 120.);
        crate::physics::simulate(&mut f, &e, 1. / 120.);
        if f.crashed || f.landed {
            break;
        }
    }
    f
}
pub fn diagnostic_entry() -> (Flight, f64, f64) {
    let mut f = Flight::spacecraft();
    f.space.stage = Stage::Capsule;
    f.fuel = 0.;
    f.space.chute_armed = true;
    f.assist = false;
    f.position = surface_point(0., 0., 120000.);
    f.velocity = DVec3::X * 10800. - f.up() * 1100.;
    f.orientation = DQuat::from_rotation_arc(DVec3::Z, f.velocity.normalize());
    let mut e = Environment::default();
    e.wind_enabled = false;
    let (mut peak_g, mut peak_heat) = (0f64, 0f64);
    for _ in 0..240000 {
        e.advance(1. / 120.);
        crate::physics::simulate(&mut f, &e, 1. / 120.);
        peak_g = peak_g.max(f.g_load);
        peak_heat = peak_heat.max(f.space.heat);
        if f.crashed || f.landed {
            break;
        }
    }
    (f, peak_g, peak_heat)
}
pub fn study() {
    let f = Flight::spacecraft();
    let (descent, ret) = future_delta_v(&f);
    let node = navigation::moon_transfer(&f).unwrap();
    let forecast = navigation::predict(&f, Some(node));
    let landed = diagnostic_landing();
    let recovery = diagnostic_recovery();
    let (entry, peak_g, peak_heat) = diagnostic_entry();
    let mut orbits = Vec::new();
    for id in [3, 9] {
        let mut s = scenario(id);
        let mut e = Environment::default();
        let before = crate::orbit::Orbit::from_flight(&s);
        let actual = coast(&mut s, &mut e, 86400.);
        let after = crate::orbit::Orbit::from_flight(&s);
        orbits.push(serde_json::json!({"body":s.space.body.label(),"seconds":actual,"periapsis_error_m":after.periapsis-before.periapsis,"apoapsis_error_m":after.apoapsis.unwrap()-before.apoapsis.unwrap()}));
    }
    let report = serde_json::json!({"model":"Fictional Apollo-inspired direct return, real-scale bodies, circular mean Moon ephemeris and patched conics. Diagnostic landing controller is not a gameplay autopilot.","wet_mass_kg":f.mass(),"transfer_delta_v_m_s":delta_v(&f),"descent_delta_v_m_s":descent,"return_delta_v_m_s":ret,"planned_tli_delta_v_m_s":node.dv.length(),"planned_tli_eta_s":node.eta(&f),"predicted_moon_encounter":forecast.encounters.iter().any(|x|x.0==Body::Moon),"closest_predicted_moon_altitude_m":forecast.closest_moon,"orbital_coasts":orbits,"landing":{"landed":landed.landed,"crashed":landed.crashed,"seconds":landed.elapsed,"fuel_kg":landed.fuel,"agl_m":landed.agl(),"vertical_m_s":landed.vertical_speed(),"leg_compression_m":landed.space.legs},"parachute_recovery":{"landed":recovery.landed,"crashed":recovery.crashed,"seconds":recovery.elapsed,"deployment":recovery.space.chute,"shield_remaining":recovery.space.shield}});
    let mut report = report;
    let (moon_position, moon_velocity, _, _) = moon_inertial(f.elapsed);
    let plane_alignment = crate::orbit::Orbit::from_flight(&f)
        .angular_momentum
        .normalize()
        .dot(moon_position.cross(moon_velocity).normalize());
    report["initial_moon_plane_error_degrees"] =
        serde_json::json!(plane_alignment.clamp(-1.0, 1.0).acos().to_degrees());
    report["planned_tli_normal_delta_v_m_s"] = serde_json::json!(node.dv.z);
    report["high_speed_reentry"] = serde_json::json!({"landed":entry.landed,"crashed":entry.crashed,"seconds":entry.elapsed,"peak_g":peak_g,"peak_heat_w_m2":peak_heat,"shield_remaining":entry.space.shield,"heat_load_j_m2":entry.space.heat_load});
    std::fs::write(
        "tests/lunar-study.json",
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    assert!(
        landed.landed
            && !landed.crashed
            && recovery.landed
            && !recovery.crashed
            && entry.landed
            && !entry.crashed
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_orbital_attitude_targets_use_primary_relative_axes_and_drive_rcs() {
        for scenario_id in [3, 9] {
            let f = scenario(scenario_id);
            let (r, v) = relative(&f);
            for (mode, want) in [
                (1, v),
                (2, -v),
                (3, r),
                (6, -r),
                (4, r.cross(v)),
                (7, -r.cross(v)),
            ] {
                let mut f = f.clone();
                f.sas_mode = mode;
                assert!(attitude_direction(&f).unwrap().dot(want.normalize()) > 0.999999);
            }
        }
        for mode in 1..8 {
            let mut f = Flight::spacecraft();
            f.space.stage = Stage::Capsule;
            f.fuel = 0.;
            f.sas_mode = mode;
            f.assist = true;
            f.space.burn_direction = DVec3::new(0.3, 0.4, 0.5).normalize();
            let target = attitude_direction(&f).unwrap();
            f.orientation =
                DQuat::from_rotation_arc(-DVec3::Z, target) * DQuat::from_rotation_x(0.6);
            let initial = f.forward().dot(target);
            let mut e = Environment::default();
            for _ in 0..3600 {
                e.advance(1. / 120.);
                simulate(&mut f, &e, 1. / 120.);
            }
            assert!(
                f.forward().dot(attitude_direction(&f).unwrap()) > initial + 0.1,
                "mode {} alignment {}",
                mode,
                f.forward().dot(attitude_direction(&f).unwrap())
            );
            assert!(f.space.rcs_fuel < 400. && f.fuel == 0.);
        }
        let mut f = Flight::spacecraft();
        f.sas_mode = 5;
        assert!(attitude_direction(&f).is_none());
    }
    #[test]
    fn high_speed_capsule_entry_decelerates_heats_and_recovers() {
        let (f, peak, heat) = diagnostic_entry();
        assert!(
            f.landed && !f.crashed,
            "{} h {} speed {} heat {} g {}",
            f.status,
            f.altitude(),
            f.airspeed,
            heat,
            peak
        );
        assert!(heat > 1e5 && peak > 2. && f.space.heat_load > 1e6 && f.space.shield < 1.);
    }
    #[test]
    fn lunar_height_is_continuous_at_longitude_seam_and_poles() {
        for lat in [-1.4f64, -0.5, 0., 0.7, 1.4] {
            let a = DVec3::new(-lat.cos(), lat.sin(), 1e-12);
            let b = DVec3::new(-lat.cos(), lat.sin(), -1e-12);
            assert!((height(a) - height(b)).abs() < 0.01);
        }
        for pole in [DVec3::Y, -DVec3::Y] {
            assert!(normal(pole).is_finite() && normal(pole).dot(pole) > 0.99);
        }
    }
    #[test]
    fn landing_leg_extension_stays_bounded_and_uses_shared_geometry() {
        let mut f = scenario(10);
        let e = Environment::default();
        simulate(&mut f, &e, 0.01);
        assert_eq!(f.gear_extension, 1.);
        assert!((leg_point(&f, 0).z - 8.2).abs() < 1e-12);
        f.gear = false;
        for _ in 0..350 {
            simulate(&mut f, &e, 0.01);
        }
        assert!(f.gear_extension >= 0. && f.gear_extension < 1e-9);
        assert!((leg_point(&f, 0).z - 6.4).abs() < 1e-9);
    }
    #[test]
    fn landing_approach_and_capsule_parachute_recovery() {
        let f = diagnostic_landing();
        assert!(
            f.landed && !f.crashed,
            "{} agl {} vs {}",
            f.status,
            f.agl(),
            f.vertical_speed()
        );
        let r = diagnostic_recovery();
        assert!(
            r.landed && !r.crashed,
            "{} agl {} speed {}",
            r.status,
            r.agl(),
            r.velocity.length()
        );
        assert!(r.space.chute > 0.99);
    }
    #[test]
    fn finite_stages_have_the_required_direct_return_budget_and_lose_mass() {
        let mut f = Flight::spacecraft();
        let mass0 = f.mass();
        assert!(delta_v(&f) > 4800.);
        let (d, r) = future_delta_v(&f);
        assert!(d > 2100. && r > 3500.);
        f.throttle = 0.1;
        assert!(!stage(&mut f));
        f.throttle = 0.;
        assert!(stage(&mut f));
        assert!(f.mass() < mass0 && f.fuel == 34000. && f.space.descent_fuel == 0.);
        let m = f.mass();
        assert!(stage(&mut f));
        assert!(f.mass() < m && delta_v(&f) > 3500.);
        assert!(stage(&mut f));
        assert_eq!(f.space.stage, Stage::Capsule);
        assert_eq!(f.fuel, 0.);
        assert_eq!(delta_v(&f), 0.);
        assert!(!stage(&mut f));
        assert_eq!(f.space.debris.len(), 3);
    }
    #[test]
    fn capsule_lift_drag_moments_and_vacuum_are_consistent() {
        let mut f = Flight::spacecraft();
        f.space.stage = Stage::Capsule;
        f.orientation = DQuat::IDENTITY;
        f.mach = 8.;
        for i in 0..=180 {
            let a = (i as f64).to_radians();
            let v = DVec3::new(a.sin(), 0., a.cos()) * 3000.;
            let loads = aero(&f, v, 0.01);
            assert!(loads.force.is_finite() && loads.torque.is_finite());
            assert!(loads.force.dot(v) <= 1e-6);
        }
        let a = aero(&f, DVec3::new(300., 0., 1000.), 0.01);
        assert!(a.cl.abs() > 0.05 && a.torque.y > 0.);
        let vacuum = aero(&f, DVec3::X * 2000., 0.);
        assert_eq!(vacuum.force, DVec3::ZERO);
        assert_eq!(vacuum.torque, DVec3::ZERO);
        assert_eq!(vacuum.heat, 0.);
    }
    #[test]
    fn universal_coast_preserves_earth_and_moon_orbits() {
        for id in [3, 9] {
            let mut f = scenario(id);
            let mut e = Environment::default();
            let before = crate::orbit::Orbit::from_flight(&f);
            let time = if id == 3 { 86400. } else { 21600. };
            let actual = coast(&mut f, &mut e, time);
            assert!((actual - time).abs() < 1e-6);
            let after = crate::orbit::Orbit::from_flight(&f);
            assert!(
                (after.periapsis - before.periapsis).abs() < 0.1,
                "{id} {}",
                after.periapsis - before.periapsis
            );
            assert!((after.apoapsis.unwrap() - before.apoapsis.unwrap()).abs() < 0.1);
            assert!((e.time - f.elapsed).abs() < 1e-6);
        }
    }
    #[test]
    fn coast_stops_before_reentry_and_soi_crossing_preserves_position() {
        let mut f = Flight::spacecraft();
        f.velocity = -f.up() * 2000. - rotation().cross(f.position);
        let mut e = Environment::default();
        let advanced = coast(&mut f, &mut e, 10000.);
        assert!(advanced < 10000. && f.altitude() >= 150000. && !f.crashed);
        let (mut f, mut e) = (Flight::spacecraft(), Environment::default());
        let (c, v, _) = moon(0.);
        let n = c.normalize();
        f.position = c + n * (MOON_SOI + 100.);
        f.velocity = v - n * 1000. - rotation().cross(f.position);
        let p = f.position;
        let dt = coast(&mut f, &mut e, 30.);
        assert_eq!(f.space.body, Body::Moon);
        assert!(dt < 30.);
        assert!(f.position.distance(p) < 10000.);
    }
    #[test]
    fn lunar_touchdown_uses_spring_legs_and_hard_contact_crashes() {
        let mut f = scenario(10);
        let (c, _, q) = moon(0.);
        let site = landing_site();
        let n = q * site;
        f.position = c + n * (MOON_RADIUS + height(site) + 8.5);
        f.velocity = ground_velocity(f.position, 0., Body::Moon) - n * 0.3;
        let mut e = Environment::default();
        for _ in 0..2400 {
            e.advance(1. / 120.);
            crate::physics::simulate(&mut f, &e, 1. / 120.);
            if f.crashed {
                break;
            }
        }
        assert!(
            !f.crashed,
            "{} altitude {} compression {:?}",
            f.status,
            f.agl(),
            f.space.legs
        );
        assert!(
            f.landed && f.space.visited_moon,
            "{} vs {} legs {:?}",
            f.status,
            f.vertical_speed(),
            f.space.legs
        );
        assert!(f.space.legs.iter().filter(|x| **x > 0.).count() >= 3);
        f.throttle = 0.;
        assert!(stage(&mut f));
        let debris = f.space.debris.last().unwrap();
        let (p, _) = debris_pose(debris, f.elapsed + 1000.).unwrap();
        let (_, n0) = terrain_at(debris.position, debris.epoch);
        let (_, n1) = terrain_at(p, f.elapsed + 1000.);
        assert!(n0.distance(n1) < 1e-9);
        let mut hard = scenario(10);
        hard.position = c + n * (MOON_RADIUS + height(site) + 8.3);
        hard.velocity = ground_velocity(hard.position, 0., Body::Moon) - n * 20.;
        crate::physics::simulate(&mut hard, &e, 0.01);
        assert!(hard.crashed);
    }
    #[test]
    fn terrain_normals_are_outward_and_sloped_and_moon_has_no_aerodynamics() {
        let n = DVec3::new(1., 0.02, 0.01).normalize();
        let normal = normal(n);
        assert!(normal.dot(n) > 0.9 && normal.distance(n) > 1e-5);
        let mut f = scenario(10);
        let e = Environment::default();
        simulate(&mut f, &e, 0.01);
        assert_eq!(f.density, 0.);
        assert_eq!(f.space.heat, 0.);
        assert!(f.position.is_finite());
    }
}
