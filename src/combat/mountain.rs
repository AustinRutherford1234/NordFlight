//! Mountain free-flight and close-combat scenario variants share real geography.
use super::*;
use dogfight::Difficulty;
pub const FORCES: [(usize, usize); 5] = [(1, 0), (2, 0), (4, 0), (2, 1), (4, 1)];
pub const FORCE_LABELS: [&str; 5] = ["1 × 1", "1 × 2", "1 × 4", "2 × 2", "2 × 4"];
pub const ENTRY_LABELS: [&str; 3] = ["VALLEY MERGE", "ENEMY ON YOUR SIX", "VALLEY PURSUIT"];
#[derive(Clone, Copy, Debug)]
pub struct Setup {
    pub force: usize,
    pub entry: usize,
    pub difficulty: Difficulty,
}
impl Setup {
    pub fn id(self) -> u8 {
        20 + ((self.entry.min(2) * 5 + self.force.min(4)) * 3) as u8
            + match self.difficulty {
                Difficulty::Training => 0,
                Difficulty::Balanced => 1,
                Difficulty::Hard => 2,
            }
    }
    pub fn decode(id: u8) -> Option<Self> {
        if !(20..=64).contains(&id) {
            return None;
        }
        let index = (id - 20) as usize;
        Some(Self {
            force: index / 3 % 5,
            entry: index / 15,
            difficulty: match index % 3 {
                0 => Difficulty::Training,
                1 => Difficulty::Balanced,
                _ => Difficulty::Hard,
            },
        })
    }
}
pub fn flight(aircraft: crate::aircraft::Aircraft) -> Flight {
    let mut f = crate::scenarios::mountain_flight(aircraft);
    f.throttle = 0.8;
    f.spool = 0.8;
    f.velocity = f.forward() * 230.;
    f.airspeed = f.velocity.length();
    let (t, p, r, s) = atmosphere(f.altitude());
    f.temperature = t;
    f.pressure = p;
    f.density = r;
    f.mach = f.airspeed / s;
    f.thrust = crate::aircraft::jet_output(f.aircraft, f.spool, r, f.mach).0;
    f.status = "MOUNTAIN COMBAT / FULL LOADOUT · TERRAIN IS LIVE".into();
    f
}
pub fn start(c: &mut Combat, f: &Flight, generation: u64, setup: Setup) {
    let count = FORCES[setup.force.min(4)];
    bvr::start_close(c, f, generation, setup.difficulty, count);
    let forward = f.forward();
    let right = f.orientation * DVec3::X;
    let mut altitude = f.altitude();
    let mut poses = Vec::new();
    for i in 0..count.0 + count.1 {
        let friend = i >= count.0;
        let distance = if friend {
            -650.
        } else {
            match setup.entry {
                1 => -2200.,
                2 => 1800.,
                _ => 3500.,
            }
        };
        let lateral = if friend {
            450.
        } else {
            (i as f64 - (count.0 - 1) as f64 * 0.5) * 450.
        };
        let p = f.position + forward * distance + right * lateral;
        let dir = if friend || setup.entry != 0 {
            forward
        } else {
            -forward
        };
        // Same initial flight corridor used for player and AI; never spawn an
        // opponent inside a slope merely because it is offset from the player.
        for j in 0..=24 {
            let point = p + dir * (j as f64 * 200.);
            altitude = altitude.max(contact_height_at(point) + 450.);
        }
        poses.push((p, dir));
    }
    for (i, (t, (p, dir))) in c.targets.iter_mut().zip(poses).enumerate() {
        // Wingmen enter on stepped flight levels instead of sharing a single
        // collision plane during a symmetric close-range merge.
        let step = if i >= count.0 {
            150.
        } else {
            (i % 4) as f64 * 150.
        };
        t.position = p.normalize() * (planet_radius() + altitude + step);
        t.orientation = facing(dir, t.position.normalize());
        t.velocity = dir * 230.;
        t.speed = 230.;
        t.throttle = 0.8;
        t.brain = ai::Brain {
            id: t.brain.id,
            long_range: true,
            ..default()
        };
        ai::initialize(t);
    }
    let b = c.bvr.as_mut().unwrap();
    b.range_at_start = match setup.entry {
        0 => 3500.,
        1 => 2200.,
        _ => 1800.,
    };
    b.objective = f.position;
    b.mountain_entry = setup.entry;
    c.notify(format!(
        "MOUNTAIN COMBAT / {} / {} / FULL LOADOUT",
        FORCE_LABELS[setup.force], ENTRY_LABELS[setup.entry]
    ));
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_selected_aircraft_starts_with_consistent_air_data() {
        for aircraft in crate::aircraft::Aircraft::ALL {
            let f = flight(aircraft);
            let (t, p, r, s) = atmosphere(f.altitude());
            assert_eq!(f.aircraft, aircraft);
            assert_eq!(f.airspeed, f.velocity.length());
            assert!((f.mach - f.airspeed / s).abs() < 1e-12);
            assert_eq!((f.temperature, f.pressure, f.density), (t, p, r));
            assert_eq!(
                f.thrust,
                crate::aircraft::jet_output(aircraft, f.spool, r, f.mach).0
            );
        }
    }
    #[test]
    #[ignore = "Run alone: installs the immutable real environment for this process"]
    fn real_pack_mountain_survival_study() {
        crate::environment::initialize().unwrap();
        crate::combat::bvr::close::study();
        let data: serde_json::Value = serde_json::from_slice(
            &std::fs::read("tests/mountain-combat/terrain-study.json").unwrap(),
        )
        .unwrap();
        assert_eq!(
            data.as_array().unwrap().len(),
            if std::env::var_os("NORD_MOUNTAIN_CASE").is_some() {
                1
            } else {
                15
            }
        );
        for row in data.as_array().unwrap() {
            assert_eq!(row["finite"], true, "{row}");
            if std::env::var("NORD_MOUNTAIN_WEAPONS").as_deref() != Ok("1") {
                assert_eq!(row["player_crashed"], false, "{row}");
                assert!(row["crashed_ai"].as_array().unwrap().is_empty(), "{row}");
            }
        }
    }
    #[test]
    fn all_fifteen_setups_have_unique_ids_and_difficulty_roundtrips() {
        let mut ids = std::collections::HashSet::new();
        for force in 0..5 {
            for entry in 0..3 {
                for difficulty in [Difficulty::Training, Difficulty::Balanced, Difficulty::Hard] {
                    let s = Setup {
                        force,
                        entry,
                        difficulty,
                    };
                    assert!(ids.insert(s.id()));
                    let r = Setup::decode(s.id()).unwrap();
                    assert_eq!((r.force, r.entry, r.difficulty), (force, entry, difficulty));
                }
            }
        }
        assert_eq!(ids.len(), 45);
        assert!(Setup::decode(19).is_none());
        assert!(Setup::decode(65).is_none());
    }
    #[test]
    fn force_sizes_start_with_full_player_stores_and_correct_teams() {
        for force in 0..5 {
            for entry in 0..3 {
                let f = flight(crate::aircraft::Aircraft::Gripen);
                let mut c = Combat::default();
                start(
                    &mut c,
                    &f,
                    1,
                    Setup {
                        force,
                        entry,
                        difficulty: Difficulty::Balanced,
                    },
                );
                let b = c.bvr.as_ref().unwrap();
                assert_eq!((b.enemies, b.allies), FORCES[force]);
                assert_eq!(c.ammo, f.aircraft.spec().ammo);
                assert_eq!(c.targets.len(), FORCES[force].0 + FORCES[force].1);
                for t in &c.targets {
                    assert!(t.dynamics.agl() > 400.);
                    assert!(t.position.is_finite());
                }
            }
        }
    }
}
