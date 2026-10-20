//! AI pilots command the same Flight plant as the player. No velocity/attitude steering.
use super::*;
use dogfight::Difficulty;

#[derive(Clone, Default)]
pub struct Brain {
    pub id: usize,
    pub long_range: bool,
    pub mode: &'static str,
    pub memory: Option<(DVec3, DVec3, f64)>,
    pub observed: bool,
    pub elapsed: f64,
    pub input: DVec3,
    pub physics_steps: u64,
}
pub fn airframe() -> Flight {
    let mut f = flight();
    f.gear_extension = 0.;
    f.gear_door = 0.;
    f.gear_phase = 0;
    f.flaps = 0.;
    f.landed = false;
    f.autopilot = false;
    f.infinite_fuel = false;
    f.paused = false;
    f
}
pub fn skill(d: Difficulty) -> f64 {
    match d {
        Difficulty::Training => 0.45,
        Difficulty::Balanced => 0.75,
        Difficulty::Hard => 1.,
    }
}
/// Detection quality is a game sensor approximation, not access to target identity.
pub fn visible(t: &Target, contact: guidance::Contact, sun: DVec3) -> bool {
    if !contact.alive || !guidance::terrain_visible(t.position, contact.position) {
        return false;
    }
    let r = contact.position - t.position;
    let range = r.length();
    let cosine = (t.orientation * -DVec3::Z).dot(r.normalize_or_zero());
    let visual = range < 9000.
        && cosine > -0.35
        && !(sun.dot(r.normalize_or_zero()) > 0.996 && range > 1500.);
    let local = t.orientation.conjugate() * r;
    let az = local.x.atan2(-local.z).to_degrees();
    let elevation = local.y.atan2(local.x.hypot(local.z)).to_degrees();
    let sweep =
        ((t.brain.elapsed * 32. + t.brain.id as f64 * 27.).rem_euclid(240.) - 120.).abs() - 60.;
    let known = t.brain.memory.is_some_and(|(p, _, at)| {
        t.brain.elapsed - at < 3.
            && (p - t.position)
                .normalize_or_zero()
                .dot(r.normalize_or_zero())
                > 8_f64.to_radians().cos()
    });
    let radar = (known || (az - sweep).abs() < 8.)
        && elevation.abs() < 45.
        && t.damage.radar > 0.25
        && t.damage.avionics > 0.25
        && range < 170000.
        && cosine > 65_f64.to_radians().cos()
        && (if t.brain.long_range {
            bvr::radar_signal(t.position, contact)
        } else {
            guidance::quality(Weapon::Radar, t.position, contact, sun).0
        }) > 0.18;
    visual || radar
}
pub fn warning(t: &Target, m: &Missile, sun: DVec3) -> bool {
    if !m.alive || t.damage.avionics < 0.25 {
        return false;
    }
    let r = m.position - t.position;
    let closing = (m.velocity - t.velocity).dot(-r.normalize_or_zero());
    if closing <= 0. || !guidance::terrain_visible(t.position, m.position) {
        return false;
    }
    // RF warning from a radiating seeker; optical warning from a close, burning
    // missile in view. Passive midcourse and unobserved IR shots remain unknown.
    let rf = m.kind == Weapon::Radar
        && m.gnc.active
        && r.length() < 26000.
        && m.gnc.gimbal.normalize_or_zero().dot(-r.normalize_or_zero()) > 0.5;
    let optical = r.length() < 6000.
        && m.age < m.kind.burn() + 0.18
        && (t.orientation * -DVec3::Z).dot(r.normalize_or_zero()) > -0.5
        && sun.dot(r.normalize_or_zero()) < 0.998;
    rf || optical
}
pub fn rf_bearing(t: &Target, f: &Flight, powered: bool) -> Option<DVec3> {
    let r = t.position - f.position;
    (powered
        && f.battery
        && f.damage.radar > 0.25
        && t.damage.avionics > 0.25
        && r.length() < 240000.
        && f.forward().dot(r.normalize_or_zero()) > 65_f64.to_radians().cos()
        && guidance::terrain_visible(t.position, f.position))
    .then(|| -r.normalize_or_zero())
}
pub fn sense_player(t: &mut Target, f: &Flight, d: Difficulty, sun: DVec3, dt: f64) {
    t.brain.elapsed += dt;
    t.brain.observed = visible(t, guidance::player(f), sun);
    if t.brain.observed {
        // Deterministic measurement error affects awareness, never the flight RNG.
        let lag = 0.12 + (1. - skill(d)) * 0.8;
        let error = (3. + (1. - skill(d)) * 30.) * (t.brain.elapsed * 0.37).sin();
        t.brain.memory = Some((
            f.position - f.velocity * lag + f.up() * error,
            f.velocity,
            t.brain.elapsed,
        ));
    }
    if t.brain.memory.is_some_and(|m| t.brain.elapsed - m.2 > 15.) {
        t.brain.memory = None;
    }
}
pub fn avoidance(position: DVec3, velocity: DVec3, other: &[(DVec3, DVec3)]) -> DVec3 {
    avoidance_iter(position, velocity, other.iter().copied())
}
pub fn avoidance_iter(
    position: DVec3,
    velocity: DVec3,
    other: impl Iterator<Item = (DVec3, DVec3)>,
) -> DVec3 {
    let mut correction = DVec3::ZERO;
    for (p, v) in other {
        let r = p - position;
        let dv = v - velocity;
        if r.length_squared() < 1. {
            continue;
        }
        let tau = (-r.dot(dv) / dv.length_squared().max(1.)).clamp(0., 5.);
        if (r + dv * tau).length() < 180. && r.length() < 2200. {
            correction -= r.normalize_or_zero() * (1. - tau / 6.);
        }
    }
    correction
}
pub fn sync(t: &mut Target) {
    let f = &mut t.dynamics;
    f.position = t.position;
    f.velocity = t.velocity;
    f.orientation = t.orientation;
    f.damage = t.damage.clone();
    f.integrity = t.health;
    f.fuel = t.fuel;
    f.crashed = !t.alive;
    f.paused = false;
}
pub fn initialize(t: &mut Target) {
    t.dynamics = airframe();
    sync(t);
    let f = &mut t.dynamics;
    f.throttle = t.throttle;
    f.spool = t.throttle;
    let (_, _, rho, sound) = atmosphere(f.altitude());
    f.density = rho;
    f.airspeed = f.velocity.length();
    f.mach = f.airspeed / sound;
}
/// Explicit command boundary: only throttle and controls may be changed by a pilot.
pub fn command(f: &mut Flight, desired: DVec3, speed: f64, d: Difficulty, dt: f64) -> &'static str {
    let up = f.up();
    let forward = f.forward();
    let right = forward.cross(up).normalize_or(DVec3::X);
    let horizontal = (forward - up * forward.dot(up)).normalize_or(forward);
    let heading = desired.dot(right).atan2(desired.dot(horizontal));
    let air = f.velocity - f.wind;
    let airspeed = air.length();
    let path = air.dot(up).atan2((air - up * air.dot(up)).length().max(1.));
    let recovery = f.stall() || f.alpha > 0.34 || airspeed < 100.;
    let clearance = f.agl();
    let lookahead = terrain_height(
        (f.position + f.velocity * 5.).x,
        (f.position + f.velocity * 5.).z,
    );
    let terrain =
        clearance < 450. + (-f.vertical_speed()).max(0.) * 5. || f.altitude() < lookahead + 450.;
    let bank = if recovery || terrain {
        0.
    } else {
        let limit = if airspeed > 220. && f.alpha.abs() < 0.20 {
            1.45
        } else {
            1.20
        };
        (heading * 1.3).clamp(-limit, limit)
    };
    let roll_error = (bank - f.roll() + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
        - std::f64::consts::PI;
    let target_path = if recovery {
        -0.10
    } else if terrain {
        0.24
    } else {
        desired.dot(up).clamp(-0.3, 0.3).asin()
    };
    // Lift required for a coordinated bank plus vertical-path correction.
    let desired_g = if recovery {
        0.3
    } else {
        (1. / f.roll().cos().max(0.12) + (target_path - path) * 4.).clamp(-1., 8.5)
    };
    let pitch = ((desired_g - f.g_load) * 0.14 + (target_path - path) * 0.5).clamp(-0.7, 1.);
    let roll = (roll_error * 2.4).clamp(-1., 1.);
    let yaw = (-f.beta * 1.8).clamp(-0.4, 0.4);
    let target = DVec3::new(pitch, yaw, roll);
    let response = if recovery || terrain { 1. } else { skill(d) };
    let tau = 0.09 + (1. - response) * 0.45;
    f.controls += (target - f.controls) * (1. - (-dt / tau).exp());
    f.assist = f.damage.avionics > 0.25;
    f.autopilot = false;
    f.airbrake = !recovery && !terrain && airspeed > speed + 45. && heading.abs() < 0.5;
    let throttle = if recovery || terrain {
        1.
    } else {
        let drag = 0.5 * f.density * airspeed * airspeed * WING_AREA * f.aero_coeff.drag.max(0.);
        let requested = drag + f.mass() * ((speed - airspeed) * 0.3 + 9.81 * path.sin());
        let engine = crate::propulsion::JetConditions::new(f.density, f.mach);
        let (mut lo, mut hi) = (0., 1.);
        for _ in 0..10 {
            let mid = (lo + hi) * 0.5;
            if engine.output(mid).0 * f.damage.engine < requested {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        (lo + hi) * 0.5
    };
    f.throttle += (throttle - f.throttle) * (1. - (-dt / 0.25).exp());
    if recovery {
        "STALL RECOVERY"
    } else if terrain {
        "TERRAIN RECOVERY"
    } else {
        "TRACK"
    }
}
pub fn fly(t: &mut Target, e: &Environment, desired: DVec3, speed: f64, d: Difficulty, dt: f64) {
    let _measure = crate::performance::span(13);
    if dt <= 0. {
        return;
    }
    sync(t);
    // Diagnostic callers can request coarse ticks. Subdivide exactly as flight,
    // applying the pilot at every physics step, never reducing distant fidelity.
    let mut left = dt;
    let requested_mode = match t.brain.mode {
        "STALL RECOVERY" | "TERRAIN RECOVERY" | "" => "TRACK",
        other => other,
    };
    while left > 1e-10 {
        let h = left.min(1. / 120.);
        if !t.dynamics.crashed {
            let safety = command(
                &mut t.dynamics,
                desired.normalize_or(t.orientation * -DVec3::Z),
                speed,
                d,
                h,
            );
            t.brain.mode = if safety != "TRACK" {
                safety
            } else {
                requested_mode
            };
        }
        simulate(&mut t.dynamics, e, h);
        t.brain.physics_steps += 1;
        left -= h;
    }
    let f = &t.dynamics;
    t.position = f.position;
    t.velocity = f.velocity;
    t.orientation = f.orientation;
    t.damage = f.damage.clone();
    t.health = f.integrity;
    t.fuel = f.fuel;
    t.throttle = f.spool;
    t.alive = !f.crashed;
    t.wreck_age = f.wreck_age;
    t.brain.input = f.controls;
}

pub fn study() {
    use std::io::Write;
    let mut csv =
        std::io::BufWriter::new(std::fs::File::create("tests/ai-flight-study.csv").unwrap());
    writeln!(csv,"case,time,mode,altitude,airspeed,mach,g,alpha,bank,pitch,throttle,fuel,pitch_input,yaw_input,roll_input,energy").unwrap();
    let mut results = Vec::new();
    for case in ["level", "turn", "stall", "engine-out", "jammed", "detached"] {
        let mut player = flight();
        player.position = surface_point(0., -1500., 5000.);
        let mut c = Combat::default();
        c.start(&player, 123);
        let t = &mut c.targets[0];
        t.position = player.position;
        t.velocity = player.forward() * 250.;
        t.orientation = player.orientation;
        if case == "engine-out" {
            t.damage.engine = 0.;
        }
        if case == "jammed" {
            t.damage.surface_state[2] = damage::SurfaceState::Jammed(0.20);
        }
        if case == "detached" {
            t.damage.surface_state[2] = damage::SurfaceState::Detached;
            t.damage.surfaces[2] = 0.;
        }
        if case == "stall" {
            t.velocity = player.forward() * 85.;
            t.orientation = (player.orientation * DQuat::from_rotation_x(0.6)).normalize();
        }
        let direction = if case == "turn" {
            (player.forward() * 0.5 + player.orientation * DVec3::X * 0.8).normalize()
        } else {
            (player.forward() - player.up() * player.forward().dot(player.up())).normalize()
        };
        let mut e = Environment::default();
        e.wind_enabled = false;
        let mut peak_g = 0_f64;
        let mut peak_alpha = 0_f64;
        let mut min_speed = f64::INFINITY;
        let wall = std::time::Instant::now();
        for step in 0..120 * 60 {
            e.advance(1. / 120.);
            fly(t, &e, direction, 250., Difficulty::Hard, 1. / 120.);
            let f = &t.dynamics;
            peak_g = peak_g.max(f.g_load.abs());
            peak_alpha = peak_alpha.max(f.alpha.abs());
            min_speed = min_speed.min(f.airspeed);
            if step % 60 == 0 {
                writeln!(csv,"{case},{:.3},{},{:.3},{:.3},{:.4},{:.4},{:.5},{:.5},{:.5},{:.4},{:.3},{:.4},{:.4},{:.4},{:.3}",f.elapsed,t.brain.mode,f.altitude(),f.airspeed,f.mach,f.g_load,f.alpha,f.roll(),f.pitch(),f.throttle,f.fuel,f.controls.x,f.controls.y,f.controls.z,f.velocity.length_squared()/2.+9.81*f.altitude()).unwrap();
            }
            if !t.alive {
                break;
            }
        }
        results.push(serde_json::json!({"case":case,"alive":t.alive,"altitude":t.dynamics.altitude(),"airspeed":t.dynamics.airspeed,"alpha":t.dynamics.alpha,"bank":t.dynamics.roll(),"heading_error_deg":t.dynamics.forward().dot(direction).clamp(-1.,1.).acos().to_degrees(),"peak_g":peak_g,"peak_alpha":peak_alpha,"min_speed":min_speed,"fuel":t.fuel,"thrust":t.dynamics.thrust,"physics_steps":t.brain.physics_steps,"wall_seconds":wall.elapsed().as_secs_f64()}));
    }
    csv.flush().unwrap();
    std::fs::write(
        "tests/ai-flight-study.json",
        serde_json::to_string_pretty(&results).unwrap(),
    )
    .unwrap();
    println!("AI flight diagnostics: tests/ai-flight-study.json / .csv");
    combat_study();
}

fn combat_study() {
    use std::io::Write;
    let mut csv =
        std::io::BufWriter::new(std::fs::File::create("tests/ai-combat-flight.csv").unwrap());
    writeln!(csv,"scenario,difficulty,mission,time,aircraft,mode,altitude,airspeed,mach,g,alpha,bank,throttle,fuel,pitch_input,yaw_input,roll_input,alive,physics_steps").unwrap();
    let mut reports = Vec::new();
    for scenario in 0..8 {
        let bvr_mode = scenario >= 4;
        for difficulty in [Difficulty::Training, Difficulty::Balanced, Difficulty::Hard] {
            if scenario == 0 && difficulty != Difficulty::Hard {
                continue;
            }
            if (1..4).contains(&scenario)
                && scenario
                    != match difficulty {
                        Difficulty::Training => 1,
                        Difficulty::Balanced => 2,
                        Difficulty::Hard => 3,
                    }
            {
                continue;
            }
            for defense in [false, true] {
                if !bvr_mode && defense {
                    continue;
                }
                let mut p = if bvr_mode { bvr::flight() } else { flight() };
                p.paused = false;
                let mut c = Combat::default();
                if bvr_mode {
                    let mut menu = bvr::Menu::default();
                    menu.formation = scenario - 4;
                    menu.difficulty = difficulty;
                    menu.defense = defense;
                    bvr::start(&mut c, &p, 777, &menu);
                } else if scenario == 0 {
                    c.start(&p, 777);
                } else {
                    c.start_air(&p, 777, scenario as u8 + 4);
                }
                let mut e = Environment::default();
                let goal = p.forward();
                let ticks = if bvr_mode { 180 * 120 } else { 120 * 120 };
                let mut peak_g = 0_f64;
                let mut finite = true;
                let mut modes = std::collections::BTreeMap::new();
                let wall = std::time::Instant::now();
                for step in 0..ticks {
                    e.advance(1. / 120.);
                    command(&mut p, goal, 290., Difficulty::Hard, 1. / 120.);
                    simulate(&mut p, &e, 1. / 120.);
                    c.environment = e.clone();
                    c.sun = e.sun_direction();
                    c.tick(&mut p, 1. / 120.);
                    for (i, t) in c.targets.iter().enumerate() {
                        finite &= t.position.is_finite()
                            && t.velocity.is_finite()
                            && t.dynamics.rates.is_finite();
                        if t.alive {
                            peak_g = peak_g.max(t.dynamics.g_load.abs());
                        }
                        if step % 60 == 0 {
                            *modes.entry(t.brain.mode).or_insert(0usize) += 1;
                            let f = &t.dynamics;
                            writeln!(csv,"{scenario},{},{defense},{:.3},{i},{},{:.3},{:.3},{:.4},{:.4},{:.5},{:.5},{:.4},{:.3},{:.4},{:.4},{:.4},{},{}",difficulty.label(),step as f64/120.,t.brain.mode,f.altitude(),f.airspeed,f.mach,f.g_load,f.alpha,f.roll(),f.throttle,f.fuel,f.controls.x,f.controls.y,f.controls.z,t.alive,t.brain.physics_steps).unwrap();
                        }
                    }
                }
                reports.push(serde_json::json!({"scenario":scenario,"difficulty":difficulty.label(),"defense":defense,"aircraft":c.targets.len(),"simulated_seconds":ticks as f64/120.,"wall_seconds":wall.elapsed().as_secs_f64(),"finite":finite,"alive":c.targets.iter().filter(|t|t.alive).count(),"player_alive":!p.crashed,"peak_g":peak_g,"physics_steps":c.targets.iter().map(|t|t.brain.physics_steps).collect::<Vec<_>>(),"modes":modes,"missiles":c.missiles.len()+c.fight.as_ref().map_or(0,|f|f.enemy_missiles.len()),"fuel":c.targets.iter().map(|t|t.fuel).collect::<Vec<_>>()}));
            }
        }
    }
    csv.flush().unwrap();
    std::fs::write(
        "tests/ai-combat-study.json",
        serde_json::to_string_pretty(&reports).unwrap(),
    )
    .unwrap();
    println!("AI integrated diagnostics: tests/ai-combat-study.json / ai-combat-flight.csv");
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target() -> Target {
        let mut c = Combat::default();
        c.start(&flight(), 12);
        c.targets.remove(0)
    }
    #[test]
    fn ai_uses_identical_player_plant_for_every_difficulty_and_surface_failure() {
        let mut e = Environment::default();
        e.wind_enabled = false;
        for difficulty in [Difficulty::Training, Difficulty::Balanced, Difficulty::Hard] {
            for failed in [false, true] {
                let mut t = target();
                if failed {
                    t.damage.surface_state[2] = damage::SurfaceState::Jammed(0.2);
                    t.damage.surface_state[5] = damage::SurfaceState::Detached;
                    t.damage.surfaces[5] = 0.;
                }
                sync(&mut t);
                let mut player = t.dynamics.clone();
                let desired = (player.forward() + player.orientation * DVec3::X * 0.4).normalize();
                for _ in 0..120 {
                    command(&mut player, desired, 260., difficulty, 1. / 120.);
                    simulate(&mut player, &e, 1. / 120.);
                    fly(&mut t, &e, desired, 260., difficulty, 1. / 120.);
                    assert!(t.velocity.distance(player.velocity) < 1e-10);
                    assert!(t.position.distance(player.position) < 1e-9);
                    assert!(t.dynamics.rates.distance(player.rates) < 1e-10);
                    assert_eq!(t.fuel, player.fuel);
                }
            }
        }
    }
    #[test]
    fn pilot_commands_never_set_position_velocity_orientation_or_rates() {
        let mut f = airframe();
        let before = f.clone();
        command(&mut f, DVec3::X, 300., Difficulty::Hard, 1. / 120.);
        assert_eq!(f.position, before.position);
        assert_eq!(f.velocity, before.velocity);
        assert_eq!(f.orientation, before.orientation);
        assert_eq!(f.rates, before.rates);
        assert_eq!(f.fuel, before.fuel);
        assert_eq!(f.integrity, before.integrity);
        assert!(f.controls.length() > 0.);
    }
    #[test]
    fn engine_failure_removes_thrust_without_speed_floor_and_substeps_match() {
        let mut t = target();
        t.damage.engine = 0.;
        let desired = t.orientation * -DVec3::Z;
        let mut divided = t.clone();
        let e = Environment::default();
        fly(&mut t, &e, desired, 300., Difficulty::Hard, 1. / 30.);
        for _ in 0..4 {
            fly(&mut divided, &e, desired, 300., Difficulty::Hard, 1. / 120.);
        }
        assert!(t.velocity.distance(divided.velocity) < 1e-9);
        assert_eq!(t.brain.physics_steps, 4);
        assert_eq!(t.dynamics.thrust, 0.);
        assert!(!t.dynamics.engine);
    }
    #[test]
    fn sensors_do_not_reveal_unseen_player_and_memory_expires() {
        let mut t = target();
        t.damage.radar = 0.;
        let mut p = flight();
        p.position = t.position - (t.orientation * -DVec3::Z) * 20000.;
        sense_player(&mut t, &p, Difficulty::Hard, DVec3::Y, 1. / 120.);
        assert!(!t.brain.observed);
        assert!(t.brain.memory.is_none());
        t.brain.memory = Some((p.position, p.velocity, 0.));
        sense_player(&mut t, &p, Difficulty::Hard, DVec3::Y, 16.);
        assert!(t.brain.memory.is_none());
        assert!(rf_bearing(&t, &p, false).is_none());
    }
    #[test]
    fn recovery_and_turn_controllers_complete_sixty_seconds_without_magic_motion() {
        for stalled in [false, true] {
            let mut t = target();
            t.position = surface_point(0., -1500., 5000.);
            let forward = t.orientation * -DVec3::Z;
            t.velocity = forward * if stalled { 85. } else { 250. };
            if stalled {
                t.orientation = (t.orientation * DQuat::from_rotation_x(0.6)).normalize();
            }
            let desired = if stalled {
                forward
            } else {
                (forward * 0.5 + t.orientation * DVec3::X * 0.8).normalize()
            };
            let e = Environment::default();
            let mut peak = 0_f64;
            for _ in 0..7200 {
                fly(&mut t, &e, desired, 250., Difficulty::Hard, 1. / 120.);
                peak = peak.max(t.dynamics.g_load.abs());
                assert!(t.alive);
                assert!(t.dynamics.rates.is_finite());
            }
            assert!(t.dynamics.forward().dot(desired) > 10_f64.to_radians().cos());
            assert!((t.dynamics.airspeed - 250.).abs() < 10.);
            assert!(t.dynamics.alpha.abs() < 0.15);
            assert!(peak < 9.5);
        }
    }
    #[test]
    fn all_eleven_aircraft_get_same_physics_frequency_and_pause_reset_clear_state() {
        let mut p = bvr::flight();
        p.paused = false;
        let mut c = Combat::default();
        let mut menu = bvr::Menu::default();
        menu.formation = 3;
        bvr::start(&mut c, &p, 42, &menu);
        for _ in 0..120 {
            c.tick(&mut p, 1. / 120.);
        }
        assert_eq!(c.targets.len(), 11);
        for t in &c.targets {
            assert_eq!(t.brain.physics_steps, 120);
            assert!(t.alive);
        }
        let before = c.targets[0].clone();
        p.paused = true;
        c.tick(&mut p, 1.);
        assert_eq!(c.targets[0].position, before.position);
        assert_eq!(c.targets[0].fuel, before.fuel);
        assert_eq!(c.targets[0].brain.physics_steps, 120);
        bvr::start(&mut c, &p, 43, &menu);
        for t in &c.targets {
            assert_eq!(t.brain.physics_steps, 0);
            assert_eq!(t.dynamics.rates, DVec3::ZERO);
        }
    }
    #[test]
    fn avoidance_is_a_control_goal_and_rear_passive_missile_is_not_known() {
        let t = target();
        let dir = t.orientation * -DVec3::Z;
        let turn = avoidance(
            t.position,
            dir * 250.,
            &[(t.position + dir * 500., -dir * 250.)],
        );
        assert!(turn.dot(dir) < 0.);
        let mut c = Combat::default();
        let p = flight();
        c.start(&p, 1);
        c.lock = 1.;
        assert!(c.fire(&p));
        let m = &mut c.missiles[0];
        m.position = t.position - dir * 1000.;
        m.velocity = dir * 450.;
        m.kind = Weapon::Radar;
        m.gnc.active = false;
        m.age = 1.;
        assert!(!warning(&t, m, DVec3::Y));
        m.gnc.active = true;
        m.gnc.gimbal = dir;
        assert!(warning(&t, m, DVec3::Y));
    }
    #[test]
    fn bvr_search_uses_long_range_radar_beam_and_can_retain_a_track() {
        let mut t = target();
        t.position = surface_point(0., -1500., 11000.);
        t.brain.long_range = true;
        let mut contact = guidance::target(&t);
        contact.position = t.position + (t.orientation * -DVec3::Z) * 50000.;
        assert!(!visible(&t, contact, DVec3::Y));
        t.brain.elapsed = 1.875;
        assert!(visible(&t, contact, DVec3::Y));
        t.brain.memory = Some((contact.position, contact.velocity, t.brain.elapsed));
        t.brain.elapsed = 2.5;
        assert!(visible(&t, contact, DVec3::Y));
        t.damage.radar = 0.;
        assert!(!visible(&t, contact, DVec3::Y));
    }
    #[test]
    fn unseen_player_velocity_cannot_change_pursuit_of_a_coasting_contact() {
        let mut a = target();
        a.damage.radar = 0.;
        let dir = a.orientation * -DVec3::Z;
        a.brain.memory = Some((a.position + dir * 3000., dir * 230., 0.));
        let mut b = a.clone();
        let mut p = flight();
        p.position = a.position - dir * 20000.;
        p.velocity = dir * 100.;
        let mut hidden = p.clone();
        hidden.position -= dir * 10000.;
        hidden.velocity = dir * 1000.;
        let e = Environment::default();
        dogfight::advance_targets(
            std::slice::from_mut(&mut a),
            &[],
            &p,
            Difficulty::Balanced,
            DVec3::Y,
            false,
            &e,
            1. / 120.,
        );
        dogfight::advance_targets(
            std::slice::from_mut(&mut b),
            &[],
            &hidden,
            Difficulty::Balanced,
            DVec3::Y,
            false,
            &e,
            1. / 120.,
        );
        assert!(!a.brain.observed && !b.brain.observed);
        assert_eq!(a.position, b.position);
        assert_eq!(a.velocity, b.velocity);
        assert_eq!(a.dynamics.controls, b.dynamics.controls);
    }
}
