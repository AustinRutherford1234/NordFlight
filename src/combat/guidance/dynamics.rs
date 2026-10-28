//! Pressure-dependent missile weathercocking and finite control torque.
//! Static margins, damping and inertias are tunable estimates, not weapon data.
use super::*;
pub(super) fn advance(
    m: &mut Missile,
    air_axis: DVec3,
    q: f64,
    area: f64,
    speed: f64,
    aerodynamic_limit: f64,
    vector_limit: f64,
    requested_rate: DVec3,
    dt: f64,
) {
    let length: f64 = if m.gnc.aim9x {
        3.02
    } else if m.kind == Weapon::Heat {
        2.87
    } else {
        3.66
    };
    let radius: f64 = if m.gnc.aim9x {
        0.0635
    } else if m.kind == Weapon::Heat {
        0.0635
    } else {
        0.089
    };
    let mass = m.gnc.mass.max(1.);
    let inertia = DVec3::new(
        mass * length.powi(2) / 12.,
        mass * length.powi(2) / 12.,
        mass * radius.powi(2) / 2.,
    );
    let margin = if m.kind == Weapon::Heat { 0.012 } else { 0.018 };
    let stiffness = aerodynamic_limit * mass / 20_f64.to_radians() * length * margin;
    let transverse_rate = m.gnc.body_rate - m.gnc.body * m.gnc.body_rate.dot(m.gnc.body);
    let damping = q * area * length.powi(2) / (2. * speed.max(1.)) * 18.;
    let restoring = m.gnc.body.cross(air_axis) * stiffness - transverse_rate * damping;
    m.gnc.restoring_moment = restoring;
    // Separation has only aerodynamic moments and inherited rotation. The
    // attitude autopilot is introduced smoothly after clearance, never a hidden
    // body-to-wind alignment servo while the fins are supposed to be neutral.
    let authority = smoothstep((m.age - 0.22) / 0.16);
    let rate_error = requested_rate - m.gnc.body_rate;
    let transverse_inertia = inertia.x;
    let desired = (rate_error - m.gnc.body * rate_error.dot(m.gnc.body))
        * (transverse_inertia / 0.10)
        - restoring;
    let available_moment = mass * length * (0.18 * aerodynamic_limit + 0.40 * vector_limit);
    let control = desired.clamp_length_max(available_moment) * authority;
    m.gnc.control_moment = control;
    let roll_rate = m.gnc.body_rate.dot(m.gnc.body);
    let roll_damping =
        -m.gnc.body * roll_rate * (q * area * length.powi(2) / (2. * speed.max(1.)) * 0.05);
    let body_rate = m.gnc.attitude.conjugate() * m.gnc.body_rate;
    let torque = m.gnc.attitude.conjugate() * (restoring + control + roll_damping);
    let acceleration = (torque - body_rate.cross(inertia * body_rate)) / inertia;
    m.gnc.body_rate += m.gnc.attitude * acceleration * dt;
    let rotation = DQuat::from_scaled_axis(m.gnc.body_rate * dt);
    m.gnc.attitude = (rotation * m.gnc.attitude).normalize();
    m.gnc.body = m.gnc.attitude * -DVec3::Z;
}
