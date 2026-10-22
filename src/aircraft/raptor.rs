//! F-22-specific geometry and control targets. Public proportions constrain the
//! shape; aerodynamic/control derivatives remain explicit gameplay estimates.
use super::*;
use bevy::math::DMat3;

pub fn sections() -> Vec<(f64, f64, f64, f64)> {
    // z, half-width, upper half-height, vertical centre, metres. Chines widen
    // behind the cockpit; lower_profile supplies the deeper bay/engine body.
    vec![
        (-9.46, 0.012, 0.012, 0.),
        (-8.7, 0.28, 0.19, 0.),
        (-7.6, 0.58, 0.34, 0.),
        (-6.5, 0.76, 0.42, 0.02),
        (-5.4, 0.86, 0.46, 0.04),
        (-4.3, 1.10, 0.52, 0.02),
        (-3.3, 1.90, 0.59, -0.03),
        (-2.1, 2.18, 0.62, -0.06),
        (0., 2.20, 0.62, -0.04),
        (2.0, 2.12, 0.59, -0.02),
        (4.0, 1.91, 0.51, 0.),
        (5.8, 1.74, 0.43, 0.),
        (7.0, 1.61, 0.36, -0.03),
        (7.7, 1.54, 0.31, -0.05),
        (8.1, 1.42, 0.24, -0.06),
    ]
}

/// Existing upper silhouette, shared by the render skin and collision contour.
pub fn upper_profile(x: f32) -> f32 {
    let x = x.abs();
    if x < 0.58 {
        1. - 0.06 * (x / 0.58).powi(2)
    } else if x < 0.88 {
        0.94 - (x - 0.58) * 1.9
    } else {
        (0.37 - (x - 0.88) * 3.08).max(0.)
    }
}

/// Positive depth below each chine/section centre, in metres. These are
/// photo-guided exterior proportions, not inferred internal engineering data.
pub fn belly_depth(z: f32) -> f32 {
    // Interpolate the aircraft-space keel, then convert to the section frame.
    // Interpolating depths alone preserved kinks from the section-centre line.
    const STATIONS: [(f32, f32); 15] = [
        (-9.46, 0.010), (-8.7, 0.17), (-7.6, 0.36), (-6.5, 0.395),
        (-5.4, 0.45), (-4.3, 0.505), (-3.3, 0.69), (-2.1, 0.94),
        (0., 1.08), (2., 1.03), (4., 0.99), (5.8, 0.83),
        (7., 0.45), (7.7, 0.37), (8.1, 0.32),
    ];
    const CENTRES:[f32;15]=[0.,0.,0.,0.02,0.04,0.02,-0.03,-0.06,-0.04,-0.02,0.,0.,-0.03,-0.05,-0.06];
    let z = z.clamp(STATIONS[0].0, STATIONS[14].0);
    let i = STATIONS.windows(2).position(|p| z <= p[1].0).unwrap();
    let pair = &STATIONS[i..=i + 1];
    let t = (z - pair[0].0) / (pair[1].0 - pair[0].0);
    // Monotone cubic slopes remove station kinks without overshooting the keel.
    let slope = |i: usize| {
        let secant = |j: usize| (STATIONS[j + 1].1 - STATIONS[j].1) / (STATIONS[j + 1].0 - STATIONS[j].0);
        if i == 0 { return secant(0); }
        if i == STATIONS.len() - 1 { return secant(i - 1); }
        let a = secant(i - 1); let b = secant(i);
        if a * b <= 0. { 0. } else { 2. * a * b / (a + b) }
    };
    let h = pair[1].0 - pair[0].0;
    (2.*t*t*t-3.*t*t+1.)*pair[0].1 + (t*t*t-2.*t*t+t)*h*slope(i)
        + (-2.*t*t*t+3.*t*t)*pair[1].1 + (t*t*t-t*t)*h*slope(i+1)
        + CENTRES[i]*(1.-t)+CENTRES[i+1]*t
}

/// Lower body in the same section frame as upper_profile. A broad bay keel
/// blends into two engine lobes aft; the original sharp chine stays at x=1.
pub fn lower_profile(x: f32, z: f32) -> f32 {
    const X: [f32; 6] = [0., 0.25, 0.48, 0.66, 0.82, 1.];
    const BAY: [f32; 6] = [1., 0.995, 0.96, 0.84, 0.48, 0.];
    const ENGINES: [f32; 6] = [0.62, 0.91, 1., 0.91, 0.56, 0.];
    let x = x.abs().clamp(0., 1.);
    let blend = ((z - 1.1) / 2.9).clamp(0., 1.);
    let blend = blend * blend * (3. - 2. * blend);
    let i = X.windows(2).position(|p| x <= p[1]).unwrap();
    let t = (x - X[i]) / (X[i + 1] - X[i]);
    let shape = |j: usize| BAY[j] + (ENGINES[j] - BAY[j]) * blend;
    -belly_depth(z) * (shape(i) + (shape(i + 1) - shape(i)) * t)
}

#[cfg(test)]
mod nose_contour_tests {
    use super::*;
    #[test]
    fn intake_keel_has_a_gradual_continuous_slope() {
        let keel=|z:f32| {
            let sections=sections();
            let p=sections.windows(2).find(|p|z as f64<=p[1].0).unwrap();
            let t=(z as f64-p[0].0)/(p[1].0-p[0].0);
            belly_depth(z)-(p[0].3*(1.-t)+p[1].3*t)as f32
        };
        for z in [-5.4,-4.3,-3.3,-2.1] {
            let left=(keel(z)-keel(z-0.002))/0.002;
            let right=(keel(z+0.002)-keel(z))/0.002;
            assert!((left-right).abs()<0.002,"station kink at {z}");
        }
        for j in 0..100 {
            let z=-5.4+3.3*j as f32/100.;
            let slope=(keel(z+0.01)-keel(z))/0.01;
            assert!((0.02..0.26).contains(&slope),"sudden inlet rise at {z}: {slope}");
            if z< -4.4 {assert!(slope<0.085,"cockpit belly must remain a shallow slope");}
        }
        // Beneath the cockpit midpoint, substantially shallower than the first
        // fuller-belly pass; the central weapon-bay depth remains unchanged.
        assert!(belly_depth(-4.85)<0.515);
    }
}
pub fn wing(side: f32) -> [Vec3; 4] {
    [
        Vec3::new(side * 1.65, -0.04, -3.15),
        Vec3::new(side * 6.78, -0.04, 1.65),
        Vec3::new(side * 6.52, -0.04, 2.25),
        Vec3::new(side * 1.65, -0.04, 4.05),
    ]
}
pub fn panels() -> [Panel; SURFACES] {
    let mut p = [Panel::empty(); SURFACES];
    for (s, b) in [(-1., 0), (1., 4)] {
        p[b] = Panel::new(
            "STABILATOR",
            DVec3::new(s * 3.02, 0.02, 6.15),
            DVec3::Y,
            5.1,
            2.20,
            0.52,
            0.,
            -s * 0.24,
            0.,
            0.,
        );
        p[b + 1] = Panel::new(
            "FLAPERON",
            DVec3::new(s * 2.88, -0.04, 3.16),
            DVec3::Y,
            3.3,
            1.32,
            0.06,
            0.,
            -s * 0.20,
            -0.42,
            0.,
        );
        p[b + 2] = Panel::new(
            "AILERON",
            DVec3::new(s * 5.15, -0.04, 2.22),
            DVec3::Y,
            1.82,
            0.92,
            0.02,
            0.,
            -s * 0.62,
            -0.12,
            -0.35,
        );
        let cant = 29_f64.to_radians();
        p[b + 3] = Panel::new(
            "RUDDER",
            DVec3::new(s * 2.285, 1.68, 5.15),
            DVec3::new(cant.cos(), -s * cant.sin(), 0.),
            2.25,
            1.32,
            0.,
            -0.60,
            0.,
            0.,
            -s * 0.30,
        );
        p[if s < 0. { 9 } else { 10 }] = Panel::new(
            "LEADING FLAP",
            DVec3::new(s * 3.88, -0.04, -0.57),
            DVec3::Y,
            3.15,
            0.68,
            0.,
            0.,
            0.,
            0.34,
            0.,
        );
        let shape = |q: [(f64, f64, f64); 4]| Some(q.map(|(x, y, z)| DVec3::new(s * x, y, z)));
        p[b].outline = shape([
            (1.65, 0.02, 4.55),
            (4.42, 0.02, 6.0),
            (4.28, 0.02, 7.75),
            (1.65, 0.02, 7.25),
        ]);
        p[b + 1].outline = shape([
            (1.85, -0.04, 2.60),
            (3.85, -0.04, 2.30),
            (3.85, -0.04, 3.32),
            (1.85, -0.04, 4.02),
        ]);
        p[b + 2].outline = shape([
            (3.92, -0.04, 2.28),
            (6.50, -0.04, 1.90),
            (6.52, -0.04, 2.25),
            (3.92, -0.04, 3.29),
        ]);
        p[b + 3].outline = shape([
            (1.60, 0.44, 5.35),
            (2.97, 2.92, 4.95),
            (2.97, 2.92, 5.65),
            (1.60, 0.44, 6.34),
        ]);
        p[if s < 0. { 9 } else { 10 }].outline = shape([
            (1.65, -0.04, -3.15),
            (6.78, -0.04, 1.65),
            (6.50, -0.04, 1.90),
            (1.65, -0.04, -2.45),
        ]);
    }
    p
}
/// Simulator tuning: stronger stabilator loads without enlarging the mesh or
/// changing the physical hinge/travel. This is not calibrated F-22 test data.
pub const STABILATOR_EFFECTIVENESS: f64 = 1.30;

pub fn nozzle_target(engine: usize, actuator: DVec3) -> f64 {
    let side = if engine == 0 { -1. } else { 1. };
    // Common pitch plus opposing vertical thrust for roll. Each nozzle keeps
    // its own +/-20 degree limit, including combined pitch/roll commands.
    (-actuator.x + side * actuator.z).clamp(-1., 1.) * 20_f64.to_radians()
}

/// Moment derivatives about the current CG for pitch, yaw and roll commands.
/// Both nozzles still vector only in pitch; differential vertical thrust rolls
/// the aircraft. No lateral thrust or independent yaw-vector axis is added.
pub fn vector_jacobian(f: &Flight) -> DMat3 {
    if !f.engine || (!f.infinite_fuel && f.fuel <= 0.) || f.damage.hydraulics <= 0.12 {
        return DMat3::ZERO;
    }
    let thrust = jet_output(Aircraft::F22, f.spool, f.density, f.mach).0 / 2.;
    let mut columns = [DVec3::ZERO; 3];
    let cg = mass_center(f);
    for (i, pos) in f.aircraft.engine_positions().iter().enumerate() {
        if f.damage.nozzles[i] <= 0.1 || !f.systems.engine_on[i] {
            continue;
        }
        let angle = f.systems.nozzle[i];
        let derivative = DVec3::new(0., angle.cos(), angle.sin()) * thrust * f.systems.engines[i];
        for (column, axis) in columns.iter_mut().zip([DVec3::X, DVec3::Y, DVec3::Z]) {
            // Include the per-engine mixer saturation in the allocation model.
            let slope = (nozzle_target(i, f.surface + axis * 1e-4)
                - nozzle_target(i, f.surface - axis * 1e-4))
                / 2e-4;
            *column += (*pos - cg).cross(derivative * slope);
        }
    }
    DMat3::from_cols(columns[0], columns[1], columns[2])
}
pub fn target_rates(f: &Flight, command: DVec3) -> DVec3 {
    // Blend from incidence command at low speed to pitch-rate command at high
    // speed. Neutral stick unloads; full aft stick permits controlled high alpha.
    let low = if f.autopilot {
        0.
    } else {
        ((190. - f.airspeed) / 90.).clamp(0., 1.)
    };
    let alpha_target = if command.x >= 0. {
        0.06 + command.x * 0.98
    } else {
        0.06 + command.x * 0.42
    };
    let incidence_rate = ((alpha_target - f.alpha) * 1.6).clamp(-0.65, 0.75);
    let pitch = (command.x * 0.65 * (1. - low) + incidence_rate * low)
        .min(((1.08 - f.alpha - f.rates.x * 0.15) * 1.8).max(-0.4));
    let authority = (1. - 0.70 * (f.alpha.abs() / 1.05).clamp(0., 1.)).max(0.25);
    DVec3::new(
        pitch,
        command.y * 0.32 - f.beta * 0.65,
        -command.z * 2.80 * authority,
    )
}

fn trial(
    speed: f64,
    alpha: f64,
    assist: bool,
    nozzle_health: f64,
) -> (Flight, f64, f64, Vec<serde_json::Value>) {
    let mut f = crate::combat::flight();
    select(&mut f, Aircraft::F22);
    f.position = surface_point(0., 0., 6000.);
    f.velocity = DVec3::new(0., 0., -speed);
    f.orientation = bevy::math::DQuat::from_rotation_x(alpha);
    f.assist = assist;
    f.gear = false;
    f.gear_extension = 0.;
    f.flaps = 0.;
    f.throttle = 1.;
    f.spool = 1.;
    f.infinite_fuel = true;
    f.damage.nozzles = [nozzle_health; 2];
    let mut samples = Vec::new();
    let mut peak = alpha;
    let mut maxrate = 0_f64;
    for i in 0..2400 {
        f.controls = if i < 960 && speed < 200. {
            DVec3::new(0.85, 0., 0.)
        } else {
            DVec3::ZERO
        };
        crate::physics::simulate(&mut f, &Environment::default(), 1. / 120.);
        peak = peak.max(f.alpha);
        maxrate = maxrate.max(f.rates.length());
        if i % 120 == 119 {
            samples.push(serde_json::json!({"t":(i+1)as f64/120.,"alpha":f.alpha.to_degrees(),"speed":f.airspeed,"q":f.rates.x,"surface":f.surface.x,"nozzle":f.systems.nozzle[0].to_degrees(),"g":f.g_load,"height":f.altitude()}));
        }
    }
    (f, peak, maxrate, samples)
}
pub fn study() {
    let mut rows = Vec::new();
    for (name, speed, alpha, assist, nozzle_health) in [
        ("low_speed_pull", 110., 0., true, 1.),
        ("high_alpha_hold", 95., 0.70, true, 1.),
        ("high_alpha_no_vector", 95., 0.70, true, 0.),
        ("direct_control", 110., 0., false, 1.),
        ("cruise", 240., 0., true, 1.),
        ("transonic", 340., 0., true, 1.),
    ] {
        let (f, peak, maxrate, samples) = trial(speed, alpha, assist, nozzle_health);
        rows.push(serde_json::json!({"case":name,"peak_alpha":peak.to_degrees(),"max_rate":maxrate,"crashed":f.crashed,"finite":f.position.is_finite()&&f.rates.is_finite(),"samples":samples}));
    }
    for bank in [0., 0.70] {
        let (f, samples) = level_trial(bank);
        rows.push(serde_json::json!({"case":if bank==0.{"level_acceleration"}else{"banked_turn"},"finite":f.position.is_finite()&&f.rates.is_finite(),"crashed":f.crashed,"samples":samples}));
    }
    let f = landing_trial();
    rows.push(serde_json::json!({"case":"assisted_approach","landed":f.landed,"crashed":f.crashed,"sink":f.touchdown_sink,"speed":f.airspeed,"elapsed":f.elapsed,"position":[f.position.x,f.position.z],"status":f.status}));
    for (name, command, assist, engine_out) in [
        ("low_speed_roll_right", 0.6, true, false),
        ("low_speed_roll_left", -0.6, true, false),
        ("direct_roll_right", 0.6, false, false),
        ("engine_out_roll", 0.6, true, true),
    ] {
        let (f, differential, samples) = roll_trial(command, assist, engine_out);
        rows.push(serde_json::json!({"case":name,"finite":f.position.is_finite()&&f.rates.is_finite(),"crashed":f.crashed,"peak_differential_deg":differential.to_degrees(),"samples":samples}));
    }
    println!("{}", serde_json::to_string_pretty(&rows).unwrap());
}
pub(crate) fn roll_trial(
    command: f64,
    assist: bool,
    engine_out: bool,
) -> (Flight, f64, Vec<serde_json::Value>) {
    roll_case(80., 0.55, command, assist, engine_out, 120)
}
pub(crate) fn roll_case(
    speed: f64,
    alpha: f64,
    command: f64,
    assist: bool,
    engine_out: bool,
    steps: usize,
) -> (Flight, f64, Vec<serde_json::Value>) {
    let mut f = crate::combat::flight();
    select(&mut f, Aircraft::F22);
    f.position = surface_point(0., 0., 6000.);
    f.velocity = DVec3::new(0., 0., -speed);
    f.orientation = bevy::math::DQuat::from_rotation_x(alpha);
    f.gear = false;
    f.gear_extension = 0.;
    f.flaps = 0.;
    f.assist = assist;
    f.throttle = 1.;
    f.spool = 1.;
    f.infinite_fuel = true;
    f.controls.z = command;
    if engine_out {
        f.systems.engine_on[0] = false;
    }
    let mut differential = 0_f64;
    let mut samples = Vec::new();
    for step in 0..steps {
        crate::physics::simulate(&mut f, &Environment::default(), 1. / 120.);
        differential = differential.max((f.systems.nozzle[0] - f.systems.nozzle[1]).abs());
        if step % 30 == 29 {
            samples.push(serde_json::json!({"t":(step+1)as f64/120.,"roll_rate":f.rates.z,"alpha_deg":f.alpha.to_degrees(),"actuator":f.surface.to_array(),"nozzle_deg":f.systems.nozzle.map(f64::to_degrees),"speed":f.airspeed}));
        }
    }
    (f, differential, samples)
}
pub fn roll_study() {
    let mut rows = Vec::new();
    for (speed, alpha) in [(80., 0.55), (160., 0.10), (240., 0.06)] {
        for assist in [false, true] {
            for command in [-1., 1.] {
                let (f, differential, samples) =
                    roll_case(speed, alpha, command, assist, false, 240);
                rows.push(serde_json::json!({"speed":speed,"initial_alpha_deg":alpha.to_degrees(),"fbw":assist,"key":if command<0.{"Q"}else{"E"},"roll_rate_deg_s":f.rates.z.to_degrees(),"bank_deg":f.roll().to_degrees(),"peak_differential_deg":differential.to_degrees(),"finite":f.rates.is_finite()&&f.position.is_finite(),"crashed":f.crashed,"samples":samples}));
            }
        }
    }
    println!("{}", serde_json::to_string_pretty(&rows).unwrap());
}
fn level_trial(bank: f64) -> (Flight, Vec<serde_json::Value>) {
    let mut f = crate::combat::flight();
    select(&mut f, Aircraft::F22);
    f.position = surface_point(0., 0., 6000.);
    f.velocity = DVec3::new(0., 0., -240.);
    f.gear = false;
    f.gear_extension = 0.;
    f.flaps = 0.;
    f.throttle = 1.;
    f.spool = 1.;
    f.infinite_fuel = true;
    let mut samples = Vec::new();
    for i in 0..7200 {
        // Test pilot: gentle normal-load feedback rather than repeatedly feeding
        // instantaneous AoA back as a new attitude target (which excites a PIO).
        let load = (1. / f.roll().cos().max(0.5) + (6000. - f.altitude()) * 0.004
            - f.vertical_speed() * 0.04)
            .clamp(0.2, 4.);
        f.controls = DVec3::new(
            ((load - f.g_load) * 0.04 - f.rates.x * 0.35).clamp(-0.5, 0.5),
            0.,
            ((bank - f.roll()) * 2.).clamp(-0.6, 0.6),
        );
        crate::physics::simulate(&mut f, &Environment::default(), 1. / 120.);
        if i % 600 == 599 {
            samples.push(serde_json::json!({"t":(i+1)as f64/120.,"speed":f.airspeed,"mach":f.mach,"height":f.altitude(),"bank":f.roll().to_degrees(),"alpha":f.alpha.to_degrees(),"g":f.g_load,"heading":f.heading()}));
        }
    }
    (f, samples)
}
fn landing_trial() -> Flight {
    let mut f = Flight::approach();
    select(&mut f, Aircraft::F22);
    f.autopilot = true;
    f.infinite_fuel = true;
    let e = Environment::default();
    for _ in 0..18000 {
        crate::physics::simulate(&mut f, &e, 1. / 120.);
        if f.crashed || f.landed {
            break;
        }
    }
    f
}

#[cfg(test)]
mod tests {
    #[test]
    fn belly_has_bay_volume_engine_lobes_and_a_sharp_chine() {
        use super::{belly_depth, lower_profile};
        let mut added_area = 0.;
        let mut old_area = 0.;
        for i in 0..1000 {
            let x = (i as f32 + 0.5) / 1000.;
            added_area -= lower_profile(x, 0.);
            old_area += 0.83 * 0.62 * (1. - x.powi(4)).sqrt();
        }
        assert!(added_area > old_area * 1.6, "bay cross section must have meaningful additional depth");
        assert!(lower_profile(0.48, 4.) < lower_profile(0., 4.) - 0.25);
        assert!((belly_depth(0.) - 1.04).abs() < 1e-5);
        for i in 0..1800 {
            let z = -9.4 + i as f32 * 0.0097;
            assert_eq!(lower_profile(1., z), 0.);
            for x in [0., 0.25, 0.48, 0.66, 0.82, 0.99] {
                let y = lower_profile(x, z);
                assert!(y.is_finite() && y <= 0.);
                assert_eq!(y, lower_profile(-x, z));
                assert!((lower_profile(x, z + 0.001) - y).abs() < 0.002);
            }
        }
    }

    use super::*;
    use bevy::math::DQuat;
    // Steady trim through the real six-DOF force, surface and engine path.
    // These are simulator envelope checks, not calibrated F-22 performance.
    fn speed_loads(height: f64, mach: f64, power: f64, alpha: f64, pitch: f64) -> (f64, f64, f64) {
        let mut f = crate::combat::flight();
        select(&mut f, Aircraft::F22);
        f.position = surface_point(0., 0., height);
        f.orientation = DQuat::from_rotation_x(alpha);
        f.velocity = -DVec3::Z * (atmosphere(height).3 * mach);
        f.gear = false;
        f.gear_extension = 0.;
        f.gear_door = 0.;
        f.flaps = 0.;
        f.assist = false;
        f.infinite_fuel = true;
        f.throttle = power;
        f.spool = power;
        f.controls = DVec3::X * pitch;
        f.surface = f.controls;
        f.systems.nozzle = std::array::from_fn(|i| nozzle_target(i, f.surface));
        f.flow.lag_alpha = alpha;
        f.flow.attached = 1.;
        let before = f.velocity;
        let e = Environment { wind_enabled: false, ..default() };
        crate::physics::simulate(&mut f, &e, 1. / 120.);
        let acc = (f.velocity - before) * 120.;
        (acc.y + before.length_squared() / (RADIUS + height), -acc.z, f.rates.x * 120.)
    }
    fn speed_trim(height: f64, mach: f64, power: f64) -> (f64, f64, f64) {
        let trim_pitch = |alpha| {
            let (mut low, mut high) = (-0.8, 0.8);
            for _ in 0..22 {
                let mid = (low + high) * 0.5;
                if speed_loads(height, mach, power, alpha, mid).2 > 0. { high = mid; }
                else { low = mid; }
            }
            (low + high) * 0.5
        };
        let (mut low, mut high) = (-0.20, 0.25);
        for _ in 0..22 {
            let alpha = (low + high) * 0.5;
            if speed_loads(height, mach, power, alpha, trim_pitch(alpha)).0 > 0. { high = alpha; }
            else { low = alpha; }
        }
        let alpha = (low + high) * 0.5;
        let pitch = trim_pitch(alpha);
        (alpha, pitch, speed_loads(height, mach, power, alpha, pitch).1)
    }
    #[test]
    fn high_altitude_engine_envelope() {
        for power in [0.88, 1.] {
            for mach in [1.2, 1.5, 1.8, 2., 2.4, 2.5, 2.6] {
                let (alpha, pitch, acceleration) = speed_trim(15000., mach, power);
                println!("F22 envelope h=15000 power={power} M={mach} alpha={} pitch={pitch} accel={acceleration}", alpha.to_degrees());
                let residual = speed_loads(15000., mach, power, alpha, pitch);
                assert!(residual.0.abs() < 1e-4 && residual.2.abs() < 1e-4,
                    "untrimmed M={mach}: {residual:?}");
                assert!(acceleration.is_finite());
                if power == 0.88 && mach == 1.5 { assert!(acceleration > 0.2); }
                if power == 1. && mach == 2.4 { assert!(acceleration > 0.2); }
                if power == 1. && mach == 2.5 { assert!(acceleration.abs() < 0.08); }
                if power == 1. && mach == 2.6 { assert!(acceleration < -0.2); }
            }
        }
    }
    #[test]
    fn propulsion_uses_the_same_engine_deck_as_telemetry_and_allocation() {
        for mach in [0., 0.8, 1.5, 2.5, 2.8] {
            for power in [0., 0.6, 0.88, 1.] {
                let mut f = crate::combat::flight();
                select(&mut f, Aircraft::F22);
                f.spool = power;
                f.surface = DVec3::ZERO;
                f.systems.nozzle = [0.; 2];
                f.infinite_fuel = true;
                let expected = jet_output(Aircraft::F22, power, 0.194, mach).0;
                let (force, _, _) = propulsion(&mut f, 1. / 120., 0.194, mach);
                assert!((force.length() - expected).abs() < 1e-8);
                f.engine = false;
                assert_eq!(propulsion(&mut f, 1. / 120., 0.194, mach).0, DVec3::ZERO);
            }
        }
    }
    #[test]
    fn q_and_e_roll_correctly_with_fbw_on_and_off_across_the_speed_range() {
        // Keyboard Q maps to -1, E to +1. Positive body Z is a left roll.
        for (speed, alpha) in [(80., 0.55), (160., 0.10), (240., 0.06)] {
            for assist in [false, true] {
                for command in [-1., 1.] {
                    let (f, _, _) = roll_case(speed, alpha, command, assist, false, 240);
                    assert!(
                        f.rates.z * command < -0.3,
                        "speed {speed}, FBW {assist}, command {command}, rate {}",
                        f.rates.z
                    );
                    assert!(!f.crashed && f.rates.is_finite());
                    if assist && speed >= 160. {
                        assert!(
                            f.rates.z.abs() > 120_f64.to_radians(),
                            "weak FBW roll at {speed}: {}",
                            f.rates.z
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn roll_panels_agree_with_one_another_in_attached_flow() {
        let f = powered();
        let load = |u| {
            panel_loads(
                Aircraft::F22,
                DVec3::new(0., 0., -200.),
                DVec3::ZERO,
                1.,
                0.6,
                u,
                0.,
                false,
                &f.damage,
            )
        };
        let plus = load(DVec3::Z * 0.01);
        let minus = load(-DVec3::Z * 0.01);
        for i in [0, 1, 2, 4, 5, 6] {
            let derivative = (plus.surfaces[i].moment.z - minus.surfaces[i].moment.z) / 0.02;
            assert!(
                derivative > 0.,
                "panel {i}: roll moment derivative {derivative}"
            );
        }
    }
    fn powered() -> Flight {
        let mut f = crate::combat::flight();
        select(&mut f, Aircraft::F22);
        f.spool = 1.;
        f.density = 1.225;
        f.mach = 0.;
        f.infinite_fuel = true;
        f
    }
    #[test]
    fn opposing_nozzles_produce_roll_and_remain_independently_limited() {
        for roll in [-0.7, 0.7] {
            let mut f = powered();
            f.surface = DVec3::new(0., 0., roll);
            let (force, moment, _) = propulsion(&mut f, 2., 1.225, 0.);
            assert!((f.systems.nozzle[0] + f.systems.nozzle[1]).abs() < 1e-12);
            assert!(moment.z * roll > 0. && moment.z.abs() > 60000.);
            assert!(force.y.abs() < 1e-6 && (moment.x + 0.12 * force.z).abs() < 1e-6);
        }
        for pitch in [-1., -0.6, 0., 0.6, 1.] {
            for roll in [-1., 0., 1.] {
                let mut f = powered();
                f.surface = DVec3::new(pitch, 0., roll);
                propulsion(&mut f, 0.01, 1.225, 0.);
                assert!(f.systems.nozzle.iter().all(|a| a.abs() <= 0.010000001));
                propulsion(&mut f, 2., 1.225, 0.);
                assert!(
                    f.systems
                        .nozzle
                        .iter()
                        .all(|a| a.abs() <= 20_f64.to_radians())
                );
            }
        }
        let mut f = powered();
        f.surface.z = 0.6;
        propulsion(&mut f, 2., 1.225, 0.);
        let stuck = f.systems.nozzle[0];
        f.damage.nozzles[0] = 0.;
        f.surface.z = -0.6;
        propulsion(&mut f, 2., 1.225, 0.);
        assert_eq!(f.systems.nozzle[0], stuck);
        assert!(f.systems.nozzle[1] < 0.);
    }
    #[test]
    fn vector_allocation_matches_engine_moments_with_saturation_and_failures() {
        for command in [
            DVec3::ZERO,
            DVec3::new(0.2, 0., 0.3),
            DVec3::new(0.8, 0., 0.6),
        ] {
            for failure in 0..3 {
                let mut f = powered();
                f.surface = command;
                f.systems.nozzle = std::array::from_fn(|i| nozzle_target(i, command));
                if failure == 1 {
                    f.systems.engine_on[0] = false;
                }
                if failure == 2 {
                    f.damage.nozzles[1] = 0.;
                }
                let jacobian = vector_jacobian(&f);
                let moment = |u| {
                    let mut g = f.clone();
                    g.surface = u;
                    let (force, torque, _) = propulsion(&mut g, 2., f.density, f.mach);
                    torque - mass_center(&g).cross(force)
                };
                for axis in [DVec3::X, DVec3::Y, DVec3::Z] {
                    let numerical =
                        (moment(command + axis * 1e-5) - moment(command - axis * 1e-5)) / 2e-5;
                    assert!(
                        (numerical - jacobian * axis).length() < 0.01,
                        "command {command:?}, failure {failure}, axis {axis:?}"
                    );
                }
            }
        }
        let mut f = powered();
        f.engine = false;
        assert_eq!(vector_jacobian(&f), DMat3::ZERO);
        f.engine = true;
        f.damage.hydraulics = 0.;
        assert_eq!(vector_jacobian(&f), DMat3::ZERO);
        f.damage.hydraulics = 1.;
        f.fuel = 0.;
        f.infinite_fuel = false;
        assert_eq!(vector_jacobian(&f), DMat3::ZERO);
    }
    #[test]
    fn low_speed_fbw_uses_differential_vectoring_for_roll() {
        for command in [-0.6, 0.6] {
            let (f, differential, _) = roll_trial(command, true, false);
            assert!(differential > 0.03, "differential {differential}");
            assert!(f.rates.z * command < -0.06, "roll rate {}", f.rates.z);
            assert!(!f.crashed && f.rates.is_finite());
        }
        let (direct, _, _) = roll_trial(0.6, false, false);
        assert!(direct.rates.z < -0.1, "direct roll rate {}", direct.rates.z);
    }
    #[test]
    fn vector_authority_follows_power_failure_and_is_pitch_only_when_symmetric() {
        let mut f = crate::combat::flight();
        select(&mut f, Aircraft::F22);
        f.spool = 1.;
        f.density = 1.225;
        f.mach = 0.;
        let matrix = vector_jacobian(&f);
        let d = matrix.x_axis;
        assert!(d.x > 100000.);
        assert!(d.y.abs() < 1e-6 && d.z.abs() < 1e-6);
        f.damage.nozzles = [0.; 2];
        assert!(matrix.z_axis.z > 50000. && matrix.z_axis.x.abs() < 1e-6);
        assert_eq!(matrix.y_axis, DVec3::ZERO);
        assert_eq!(vector_jacobian(&f), DMat3::ZERO);
    }
    #[test]
    fn low_speed_fbw_requests_high_alpha_then_unloads() {
        let mut f = crate::combat::flight();
        select(&mut f, Aircraft::F22);
        f.airspeed = 95.;
        f.alpha = 0.7;
        assert!(target_rates(&f, DVec3::X).x > 0.);
        assert!(target_rates(&f, DVec3::ZERO).x < 0.);
    }
    #[test]
    fn unused_and_detached_panels_do_not_create_a_permanent_stall_warning() {
        let mut f = crate::combat::flight();
        select(&mut f, Aircraft::F22);
        f.flaps = 0.;
        f.velocity = DVec3::new(0., 0., -200.);
        crate::physics::simulate(&mut f, &Environment::default(), 1. / 120.);
        assert!(
            f.panel_attachment > 0.8,
            "attachment {}",
            f.panel_attachment
        );
        f.damage.surface_state[6] = crate::combat::damage::SurfaceState::Detached;
        crate::physics::simulate(&mut f, &Environment::default(), 1. / 120.);
        assert!(
            f.panel_attachment > 0.8,
            "missing panel is damage, not attached-flow telemetry"
        );
    }
    #[test]
    fn rendered_hinges_and_aerodynamic_pitch_have_the_same_physical_direction() {
        let mut f = crate::combat::flight();
        select(&mut f, Aircraft::F22);
        f.surface = DVec3::new(0.4, 0., 0.);
        let angle = angles(&f)[0];
        let rotation = surface_rotation(f.aircraft, 0, angle);
        assert!(
            (rotation * DVec3::Z).y > 0.,
            "pitch-up raises the stabilator trailing edge"
        );
        let loads = panel_loads(
            f.aircraft,
            DVec3::new(0., 0., -200.),
            DVec3::ZERO,
            1.,
            0.6,
            f.surface,
            0.,
            false,
            &f.damage,
        );
        assert!(loads.surfaces[0].force.y < 0. && loads.surfaces[0].moment.x > 0.);
        assert!(
            (surface_rotation(f.aircraft, 9, 0.3) * -DVec3::Z).y < 0.,
            "leading-edge droop points down"
        );
    }
    #[test]
    fn scheduled_leading_flaps_share_physics_angles_and_respect_a_jam() {
        let mut f = crate::combat::flight();
        select(&mut f, Aircraft::F22);
        f.flaps = 0.;
        f.alpha = 0.75;
        let air = DVec3::new(0., -100. * f.alpha.sin(), -100. * f.alpha.cos());
        let angles = angles(&f);
        let loads = panel_loads(
            f.aircraft,
            air,
            DVec3::ZERO,
            1.,
            0.3,
            DVec3::ZERO,
            0.,
            false,
            &f.damage,
        );
        assert!(angles[9] > 0.2);
        assert_eq!(angles[9], loads.surfaces[9].deflection);
        f.damage.surface_state[9] = crate::combat::damage::SurfaceState::Jammed(-0.12);
        let loads = panel_loads(
            f.aircraft,
            air,
            DVec3::ZERO,
            1.,
            0.3,
            DVec3::ONE,
            0.,
            false,
            &f.damage,
        );
        assert_eq!(loads.surfaces[9].deflection, -0.12);
    }
    #[test]
    fn raptor_polar_is_dissipative_through_stall_and_reverse_flow() {
        for alpha in [-3., -1.5, -0.8, 0., 0.4, 0.8, 1.5, 3.] {
            let mut state = crate::aerodynamics::FlowState::default();
            let c = crate::aerodynamics::evaluate_raptor(
                crate::aerodynamics::Conditions {
                    alpha,
                    beta: 0.2,
                    mach: 0.4,
                    speed: 130.,
                    density: 1.,
                    temperature: 280.,
                    flaps: 0.,
                    gear: false,
                    airbrake: false,
                    height: 1000.,
                    rates: DVec3::ZERO,
                },
                &mut state,
                1. / 120.,
            );
            let air = DVec3::new(0., -130. * alpha.sin(), -130. * alpha.cos());
            assert!(c.drag >= 0. && c.lift.is_finite());
            assert!(crate::aerodynamics::force(DVec3::X, air, 1000., c).dot(air) <= 1e-6);
        }
    }
    #[test]
    fn fbw_uses_vectoring_to_maneuver_at_high_alpha_and_unload() {
        let (f, peak, rate, _) = trial(110., 0., true, 1.);
        assert!(
            peak.to_degrees() > 45. && peak.to_degrees() < 65.,
            "peak {}",
            peak.to_degrees()
        );
        assert!(!f.crashed && f.alpha.abs() < 0.2 && rate < 1.5);
        let (healthy, peak, _, samples) = trial(95., 0.7, true, 1.);
        let (failed, _, _, jammed_samples) = trial(95., 0.7, true, 0.);
        // Peak AoA is command-limited, so a minimum healthy-versus-jammed
        // peak gap is not an authority measurement. Check use and recovery.
        assert!(
            peak.to_degrees() > 45.
                && samples
                    .iter()
                    .any(|s| s["nozzle"].as_f64().unwrap().abs() > 5.)
        );
        assert!(
            jammed_samples
                .iter()
                .all(|s| s["nozzle"].as_f64().unwrap() == 0.)
        );
        assert!(!healthy.crashed && healthy.alpha.abs() < 0.2 && !failed.crashed);
        let (_, direct, _, _) = trial(110., 0., false, 1.);
        assert!(direct > 2.0, "direct controls must permit departure");
    }
    #[test]
    fn loadout_updates_launch_geometry_balance_and_drag_without_changing_inventory() {
        let mut f = crate::combat::flight();
        select(&mut f, Aircraft::F22);
        let clean = store_drag(&f);
        let mass = f.mass();
        let p = station_for(&f, 6);
        f.systems.external_stores = true;
        assert!(store_drag(&f) > clean);
        assert_eq!(f.mass(), mass + 70.);
        assert!(station_for(&f, 6).distance(p) > 1.);
        assert!(mass_center(&f).is_finite());
        f.systems.stores[6] = false;
        assert!(mass_center(&f).x > 0.);
    }
    #[test]
    fn level_acceleration_and_sustained_bank_remain_controlled() {
        let (level, samples) = level_trial(0.);
        assert!(!level.crashed && level.airspeed > 300. && (level.altitude() - 6000.).abs() < 200.);
        for sample in samples.iter().skip(2) {
            let g = sample["g"].as_f64().unwrap();
            assert!((0.7..1.5).contains(&g), "trim load {g}");
        }
        let (turn, _) = level_trial(0.70);
        assert!(!turn.crashed && turn.position.is_finite());
        assert!((turn.roll() - 0.70).abs() < 0.15, "bank {}", turn.roll());
        assert!(
            (turn.altitude() - 6000.).abs() < 250.,
            "height {}",
            turn.altitude()
        );
    }
    #[test]
    fn takeoff_and_compliant_touchdown_use_the_selected_airframe() {
        let mut f = Flight::runway();
        select(&mut f, Aircraft::F22);
        f.throttle = 1.;
        f.infinite_fuel = true;
        let e = Environment::default();
        let mut airborne = false;
        for _ in 0..7200 {
            f.controls.x = if f.airspeed > 95. {
                ((0.18 - f.pitch()) * 2.).clamp(-0.4, 0.4)
            } else {
                0.
            };
            if !f.landed && f.agl() > 15. {
                f.gear = false;
                f.flaps = 0.;
                airborne = true;
            }
            crate::physics::simulate(&mut f, &e, 1. / 120.);
            if f.crashed {
                break;
            }
        }
        assert!(
            airborne && !f.crashed,
            "{} speed {} agl {}",
            f.status,
            f.airspeed,
            f.agl()
        );
        let mut f = Flight::runway();
        select(&mut f, Aircraft::F22);
        f.landed = false;
        f.engine = false;
        f.throttle = 0.;
        f.position += f.up() * 0.15;
        f.velocity = DVec3::new(0., -1., -80.);
        f.brakes = true;
        for _ in 0..360 {
            crate::physics::simulate(&mut f, &e, 1. / 120.);
        }
        assert!(f.landed && !f.crashed, "{}", f.status);
    }
    #[test]
    fn approach_assistance_can_land_the_raptor() {
        let f = landing_trial();
        assert!(
            f.landed && !f.crashed,
            "{} h {} v {}",
            f.status,
            f.altitude(),
            f.airspeed
        );
        assert!(
            crate::terrain::paved(f.position.x, f.position.z),
            "touchdown x {} z {}",
            f.position.x,
            f.position.z
        );
        assert!(f.touchdown_sink < 3.0);
    }
}
