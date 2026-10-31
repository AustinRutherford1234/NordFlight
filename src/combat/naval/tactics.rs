//! Mission-level objectives feed physical pilots. Sensor memory, formation feed-
//! forward and flight-envelope limits replace universal pursuit of ship truth.
use super::*;
pub fn pilots(n: &mut State, c: &mut Combat, f: &Flight, dt: f64) {
    for t in c.targets.iter_mut().filter(|t| !t.alive) {
        t.wreck_age += dt;
    }
    let mut poses = [(DVec3::ZERO, DVec3::ZERO, false); AIRCRAFT_LIMIT];
    for (i, t) in c.targets.iter().take(AIRCRAFT_LIMIT).enumerate() {
        poses[i] = (t.position, t.velocity, t.alive);
    }
    let mut leads = [None; AIRCRAFT_LIMIT];
    let mut armed = [false; AIRCRAFT_LIMIT];
    for (i, r) in n.raiders.iter().enumerate() {
        let lead = n.raiders.iter().find(|b| {
            b.raid == r.raid && b.role == Role::Bomber && c.targets[b.slot].alive && b.stores > 0
        });
        leads[i] = lead.map(|b| b.slot);
        armed[i] = lead.is_some();
    }
    let difficulty = match n.difficulty {
        0 => dogfight::Difficulty::Training,
        1 => dogfight::Difficulty::Balanced,
        _ => dogfight::Difficulty::Hard,
    };
    for (i, r) in n.raiders.iter_mut().enumerate() {
        let Some(t) = c.targets.get_mut(r.slot).filter(|t| t.alive) else {
            continue;
        };
        let ship = carrier::ship_origin(f, 0, f.elapsed);
        let up = t.position.normalize();
        let outward =
            (t.position - ship - up * (t.position - ship).dot(up)).normalize_or(-t.forward);
        if !armed[i] {
            r.withdrew = true;
        }
        let open = r.role == Role::Bomber
            && r.stores > 0
            && !r.withdrew
            && !n.ended
            && t.position.distance(ship) < 28000.;
        r.bay += ((if open { 1. } else { 0. }) - r.bay).clamp(-0.5 * dt, 0.5 * dt);
        if r.role == Role::Fighter {
            ai::sense_player(t, f, difficulty, c.sun, dt);
        }
        let known = t
            .brain
            .memory
            .filter(|(_, _, at)| t.brain.elapsed - *at < 8.);
        let intercept = known.filter(|(p, _, _)| {
            !r.withdrew && !f.crashed && !f.ejection.active && t.position.distance(*p) < 30000.
        });
        let (mut desired, mode) = if r.withdrew {
            (
                (outward + up * ((4200. - t.dynamics.altitude()) / 15000.).clamp(-0.1, 0.1))
                    .normalize(),
                "WITHDRAW",
            )
        } else if let Some((p, v, _)) = intercept {
            let time = (t.position.distance(p) / t.velocity.length().max(150.)).min(6.);
            (
                (p + v * time - t.position).normalize_or(t.forward),
                "INTERCEPT",
            )
        } else if r.role == Role::Fighter && leads[i].is_some() {
            let lead = leads[i].unwrap();
            let (p, v, _) = poses[lead];
            let forward = (v - up * v.dot(up)).normalize_or(-outward);
            let right = forward.cross(up);
            let goal = p
                + forward * 3500.
                + right * if r.slot % 2 == 0 { -1800. } else { 1800. }
                + up * 700.;
            (
                (forward + (goal - t.position) / 7000.).normalize(),
                "ESCORT",
            )
        } else {
            let goal = ship.normalize() * (RADIUS + 3500.);
            ((goal - t.position).normalize_or(-outward), "STRIKE")
        };
        let avoid = ai::avoidance_iter(
            t.position,
            t.velocity,
            poses
                .iter()
                .enumerate()
                .filter(|(j, p)| *j != r.slot && p.2)
                .map(|(_, p)| (p.0, p.1)),
        );
        desired = (desired + avoid * 1.4).normalize_or(desired);
        if let Some(m) = n
            .rounds
            .iter()
            .filter(|m| m.alive && matches!(m.source, TargetId::Ship(_)))
            .find(|m| ai::warning(t, &m.visual, c.sun))
        {
            let lateral = up.cross(m.velocity).normalize_or(outward);
            let side = if desired.dot(lateral) >= 0. {
                lateral
            } else {
                -lateral
            };
            desired = (desired + side * 0.65).normalize();
            t.brain.mode = "MISSILE DEFENSE";
        } else {
            t.brain.mode = mode;
        }
        t.brain.naval = true;
        if r.role == Role::Bomber {
            t.dynamics.bomber_stores = r.stores;
        }
        ai::fly(
            t,
            &c.environment,
            desired,
            if r.role == Role::Bomber {
                240.
            } else if intercept.is_some() {
                290.
            } else {
                250.
            },
            difficulty,
            dt,
        );
        if r.withdrew && t.position.distance(ship) > 145000. {
            t.alive = false;
        }
    }
}
/// Su-27 AI command governor. The existing physical FBW/actuator allocator
/// remains authoritative; protections reduce requests instead of moving poses.
pub fn protect(f: &mut Flight) {
    if f.ai_bomber {
        return;
    }
    f.controls.z = f.controls.z.clamp(-0.75, 0.75);
    if f.roll().abs() > 1.22 && f.controls.z * f.roll() > 0. {
        f.controls.z = 0.;
    }
    if f.g_load > 6.5 && f.controls.x > 0. {
        f.controls.x = 0.;
    }
    if f.g_load < -1. && f.controls.x < 0. {
        f.controls.x = 0.;
    }
    if f.alpha > 0.30 {
        f.controls.x = f.controls.x.min(-0.08);
        f.controls.z *= 0.3;
    }
    if f.airspeed < 140. {
        f.controls.x = f.controls.x.min(0.);
        f.controls.z *= 0.4;
        f.throttle = 1.;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raid_flies_from_120km_to_physical_release_then_withdraws_without_unbounded_turns() {
        let mut f = flight(false);
        f.crashed = true;
        let mut c = Combat::default();
        start(&mut c, &f, 77, 1);
        c.environment.wind_enabled = false;
        let mut n = c.naval.take().unwrap();
        let mut peak_bomber_g: f64 = 0.;
        let mut peak_bomber_rate: f64 = 0.;
        let mut samples = vec![];
        for k in 0..12000 {
            let dt = 0.05;
            f.elapsed += dt;
            n.clock += dt;
            pilots(&mut n, &mut c, &f, dt);
            n.hostile_launches(&c, &f);
            for r in &n.raiders {
                let t = &c.targets[r.slot];
                assert!(
                    t.alive,
                    "role {:?} k={} cause={} alt={} speed={}",
                    r.role,
                    k,
                    t.dynamics.status,
                    t.dynamics.altitude(),
                    t.dynamics.airspeed
                );
                assert!(t.velocity.length() < 600.);
                if r.role == Role::Bomber {
                    peak_bomber_g = peak_bomber_g.max(t.dynamics.g_load);
                    peak_bomber_rate = peak_bomber_rate.max(t.dynamics.rates.length());
                }
            }
            if k % 100 == 0 {
                samples.push(serde_json::json!({"t":n.clock,"aircraft":n.raiders.iter().map(|r|{let t=&c.targets[r.slot];serde_json::json!({"slot":r.slot,"role":format!("{:?}",r.role),"mode":t.brain.mode,"altitude":t.dynamics.altitude(),"speed":t.dynamics.airspeed,"g":t.dynamics.g_load,"bank":t.dynamics.roll(),"rate":t.dynamics.rates.length(),"stores":r.stores,"withdrew":r.withdrew})}).collect::<Vec<_>>()}));
            }
        }
        println!(
            "raid bomber peak_g={peak_bomber_g} peak_rate={peak_bomber_rate} rounds={}",
            n.rounds.len()
        );
        assert!(peak_bomber_g < 2.7 && peak_bomber_rate < 0.8);
        assert!(
            n.raiders
                .iter()
                .filter(|r| r.role == Role::Bomber)
                .all(|r| r.stores == 0 && r.withdrew)
        );
        std::fs::create_dir_all("tests/naval-upgrade").unwrap();
        std::fs::write(
            "tests/naval-upgrade/raid-flight.json",
            serde_json::to_vec_pretty(&samples).unwrap(),
        )
        .unwrap();
    }
}
