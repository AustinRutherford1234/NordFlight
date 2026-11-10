//! Short-range weapons for both teams in a mountain battle.
use super::*;
pub(super) fn advance(c: &mut Combat, b: &mut Battle, f: &mut Flight, _dt: f64) {
    let reaction = match b.difficulty {
        Difficulty::Training => 10.,
        Difficulty::Balanced => 6.,
        Difficulty::Hard => 4.,
    };
    let mut launches = Vec::new();
    for i in 0..c.targets.len() {
        let t = &c.targets[i];
        if !t.alive || c.clock < reaction || t.brain.mode == "TERRAIN RECOVERY" {
            continue;
        }
        let Some((id, _, _, seen)) = b.ai[i].memory else {
            continue;
        };
        if c.clock - seen > 0.5 {
            continue;
        }
        let target = id.map_or_else(|| guidance::player(f), |j| guidance::target(&c.targets[j]));
        if !target.alive || !guidance::terrain_visible(t.position, target.position) {
            continue;
        }
        let range = t.position.distance(target.position);
        let forward = t.orientation * -DVec3::Z;
        let los = (target.position - t.position).normalize_or(forward);
        let a = &mut b.ai[i];
        if a.heat > 0
            && c.clock > a.launch
            && range > 650.
            && range < 4500.
            && forward.dot(los) > 35_f64.to_radians().cos()
            && guidance::quality(Weapon::Heat, t.position, target, c.sun).0 > 0.18
        {
            launches.push((i, id));
            a.heat -= 1;
            a.launch = c.clock + 8.;
        }
        if a.gun > 0
            && c.clock > a.gun_at
            && range < 1400.
            && super::super::hud::gun_intercept(&t.dynamics,target.position,target.velocity)
                .is_some_and(|s|forward.dot(s.direction)>1.8_f64.to_radians().cos())
        {
            if let Some(fight) = &mut c.fight {
                if fight.rounds.len() < 256 {
                    let position = t.position
                        + t.orientation * crate::aircraft::gun_origin(t.dynamics.aircraft);
                    fight.rounds.push(dogfight::Round {
                        shooter: Some(i),
                        position,
                        previous: position,
                        velocity: t.velocity + forward * CANNON_MUZZLE_SPEED,
                        hostile: i < b.enemies,
                        age: 0.,
                        damage: 0.06,
                        alive: true,
                    });
                    a.gun -= 1;
                    a.gun_at = c.clock + 0.12;
                    crate::sound::emit(
                        &mut c.audio_events,
                        crate::sound::Kind::Cannon,
                        position,
                        t.velocity,
                        false,
                    );
                }
            }
        }
    }
    for (i, id) in launches {
        launch(c, b, f, Some(i), id, Weapon::Heat);
    }
}

/// Real plant, real terrain; no weapons during the terrain-survival phase.
pub fn study() {
    let mut results = Vec::new();
    let weapons = std::env::var("NORD_MOUNTAIN_WEAPONS").as_deref() == Ok("1");
    let e = Environment::default();
    let seconds = std::env::var("NORD_MOUNTAIN_SECONDS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(90)
        .clamp(10, 300);
    for force in 0..5 {
        for entry in 0..3 {
            if let Ok(case) = std::env::var("NORD_MOUNTAIN_CASE") {
                if case != format!("{force}:{entry}") {
                    continue;
                }
            }
            let setup = crate::combat::mountain::Setup {
                force,
                entry,
                difficulty: match std::env::var("NORD_MOUNTAIN_DIFFICULTY").as_deref() {
                    Ok("training") => Difficulty::Training,
                    Ok("hard") => Difficulty::Hard,
                    _ => Difficulty::Balanced,
                },
            };
            let mut f = crate::combat::mountain::flight(crate::aircraft::Aircraft::F22);
            f.paused = false;
            let mut c = Combat::default();
            crate::combat::mountain::start(&mut c, &f, 1, setup);
            let mut min_agl = f64::INFINITY;
            let mut recoveries = 0;
            let mut crashed = Vec::new();
            let mut trace = std::collections::VecDeque::new();
            for tick in 0..seconds * 120 {
                if !weapons {
                    if let Some(b) = &mut c.bvr {
                        for a in &mut b.ai {
                            a.ammo = 0;
                            a.heat = 0;
                            a.gun = 0;
                        }
                    }
                }
                // A diagnostic pilot pursues the nearest hostile using the same
                // control interface; this isn't a substitute for a human flight.
                let goal = c
                    .targets
                    .iter()
                    .take(crate::combat::mountain::FORCES[force].0)
                    .filter(|t| t.alive)
                    .min_by(|a, b| {
                        a.position
                            .distance_squared(f.position)
                            .total_cmp(&b.position.distance_squared(f.position))
                    })
                    .map_or(f.forward(), |t| {
                        (t.position - f.position).normalize_or(f.forward())
                    });
                ai::command(&mut f, goal, 250., Difficulty::Hard, 1. / 120.);
                simulate(&mut f, &e, 1. / 120.);
                c.tick(&mut f, 1. / 120.);
                if force == 4 && entry == 0 && tick % 15 == 0 {
                    if let Some(t) = c.targets.get(1) {
                        trace.push_back(serde_json::json!({"time":f.elapsed,"alive":t.alive,"agl":t.dynamics.agl(),"speed":t.dynamics.airspeed,"alpha":t.dynamics.alpha,"bank":t.dynamics.roll(),"pitch":t.dynamics.pitch(),"sink":t.dynamics.vertical_speed(),"g":t.dynamics.g_load,"controls":t.dynamics.controls.to_array(),"mode":t.brain.mode,"health":t.health,"guard":format!("{:?}",t.brain.terrain_guard)}));
                        if trace.len() > 160 {
                            trace.pop_front();
                        }
                    }
                }
                for (i, t) in c.targets.iter().enumerate() {
                    if !t.alive {
                        if !crashed.iter().any(|(j, _)| *j == i) {
                            crashed.push((i, t.dynamics.status.clone()));
                            if force == 4 && entry == 0 {
                                std::fs::create_dir_all("tests/mountain-combat").unwrap();
                                std::fs::write(
                                    "tests/mountain-combat/near-ridge-trace.json",
                                    serde_json::to_string_pretty(&trace).unwrap(),
                                )
                                .unwrap();
                            }
                        }
                    } else {
                        min_agl = min_agl.min(t.dynamics.agl());
                        if t.brain.mode == "TERRAIN RECOVERY" {
                            recoveries += 1;
                        }
                    }
                }
                if f.crashed {
                    break;
                }
                if tick % 1200 == 0 {
                    assert!(f.position.is_finite());
                }
            }
            let scans = c
                .targets
                .iter()
                .map(|t| t.brain.terrain_guard.scans)
                .sum::<u64>();
            let row = serde_json::json!({"difficulty":setup.difficulty.label(),"force":crate::combat::mountain::FORCE_LABELS[force],"entry":crate::combat::mountain::ENTRY_LABELS[entry],"seconds":f.elapsed,"minimum_ai_agl":min_agl,"recovery_steps":recoveries,"crashed_ai":crashed,"player_crashed":f.crashed,"player_status":f.status,"terrain_scans":scans,"weapons_enabled":weapons,"missiles_launched":c.missiles.len(),"gun_rounds_remaining":c.bvr.as_ref().map(|b|b.ai.iter().map(|a|a.gun as u64).sum::<u64>()),"finite":c.targets.iter().all(|t|t.position.is_finite()&&t.velocity.is_finite())});
            println!("MOUNTAIN {row}");
            results.push(row);
        }
    }
    std::fs::create_dir_all("tests/mountain-combat").unwrap();
    std::fs::write(
        "tests/mountain-combat/terrain-study.json",
        serde_json::to_string_pretty(&results).unwrap(),
    )
    .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn close_aircraft_can_launch_ir_and_fire_cannon_without_changing_the_player_inventory() {
        let mut f = crate::combat::mountain::flight(crate::aircraft::Aircraft::Gripen);
        let mut c = Combat::default();
        crate::combat::mountain::start(
            &mut c,
            &f,
            1,
            crate::combat::mountain::Setup {
                force: 0,
                entry: 0,
                difficulty: match std::env::var("NORD_MOUNTAIN_DIFFICULTY").as_deref() {
                    Ok("training") => Difficulty::Training,
                    Ok("hard") => Difficulty::Hard,
                    _ => Difficulty::Balanced,
                },
            },
        );
        c.clock = 20.;
        c.sun = -f.up();
        let mut b = c.bvr.take().unwrap();
        let t = &mut c.targets[0];
        t.position = f.position + f.forward() * 1200.;
        t.orientation = facing(-f.forward(), t.position.normalize());
        t.velocity = -f.forward() * 230.;
        ai::initialize(t);
        t.brain.mode = "TRACK";
        b.ai[0].memory = Some((None, f.position, f.velocity, c.clock));
        let ammo = c.ammo;
        let heat = b.ai[0].heat;
        let gun = b.ai[0].gun;
        advance(&mut c, &mut b, &mut f, 1. / 120.);
        assert_eq!(c.ammo, ammo);
        assert_eq!(b.ai[0].heat, heat - 1);
        assert_eq!(b.ai[0].gun, gun - 1);
        assert_eq!(c.missiles[0].kind, Weapon::Heat);
        assert_eq!(b.shots.len(), c.missiles.len());
        assert_eq!(c.fight.as_ref().unwrap().rounds[0].shooter, Some(0));
    }
}
