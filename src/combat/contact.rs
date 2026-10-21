use super::{Combat, Impact, damage};
use crate::{airframe, physics::*};
use bevy::math::{DQuat, DVec3};

pub fn advance(c: &mut Combat, f: &mut Flight, old: &[(DVec3, DQuat)], dt: f64) {
    if !c.active || f.vehicle != Vehicle::Gripen {
        return;
    }
    let player_old = f.position - f.velocity * dt;
    let mut slot = 0;
    // Player plus up to eleven AI aircraft; one contact per pair until separation.
    for a in 0..=c.targets.len() {
        for b in a + 1..=c.targets.len() {
            let current = slot;
            slot += 1;
            if a > 0 && a - 1 >= c.targets.len() || b - 1 >= c.targets.len() {
                continue;
            }
            let (pa, va, qa, oa, alive_a) = if a == 0 {
                (
                    f.position,
                    f.velocity,
                    f.orientation,
                    player_old,
                    !f.crashed,
                )
            } else {
                let t = &c.targets[a - 1];
                (t.position, t.velocity, t.orientation, old[a - 1].0, t.alive)
            };
            let tb = &c.targets[b - 1];
            let (pb, vb, qb, ob, alive_b) = (
                tb.position,
                tb.velocity,
                tb.orientation,
                old[b - 1].0,
                tb.alive,
            );
            let relative = pb - pa;
            let before = ob - oa;
            if !alive_a || !alive_b {
                continue;
            }
            let bound=if a==0{(f.aircraft.spec().length*0.6+9.).max(19.)}else{19.};
            if damage::closest_relative(before, relative).0.length() > bound {
                c.contact_pairs[current] = false;
                continue;
            }
            let mut fa = if a==0 {f.clone()}else{c.targets[a-1].dynamics.clone()};
            let mut fb = tb.dynamics.clone();
            fb.damage = tb.damage.clone();
            if a > 0 {
                fa.damage = c.targets[a - 1].damage.clone();
            }
            let hit = airframe::aircraft_contact_state(before, relative, qa, qb, &fa, &fb);
            if hit.is_none() {
                c.contact_pairs[current] = false;
                continue;
            }
            if c.contact_pairs[current] {
                continue;
            }
            c.contact_pairs[current] = true;
            let speed = (vb - va).length();
            let centre = (pa + pb) * 0.5;
            if speed >= 25.0 {
                if a == 0 {
                    crate::collision::destroy(
                        f,
                        "AIRCRAFT COLLISION",
                        relative.normalize_or_zero(),
                    );
                } else {
                    let t = &mut c.targets[a - 1];
                    t.alive = false;
                    t.health = 0.0;
                    t.damage.fire = 1.0;
                    t.wreck_age = 0.0;
                    c.hits += 1;
                }
                let t = &mut c.targets[b - 1];
                t.alive = false;
                t.health = 0.0;
                t.damage.fire = 1.0;
                t.wreck_age = 0.0;
                c.hits += 1;
                c.impacts.push(Impact {
                    position: centre,
                    age: 0.0,
                    hit: true,
                });
            } else {
                let normal = relative.normalize_or(DVec3::X);
                // Find the separating translation along the contact direction.
                let mut lo = 0.0;
                let mut hi = bound*1.5;
                for _ in 0..10 {
                    let mid = (lo + hi) * 0.5;
                    let r = relative + normal * mid;
                    if airframe::aircraft_contact_state(r, r, qa, qb, &fa, &fb).is_some() {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                let separation = (hi + 0.02) * 0.5;
                let dose = (0.06 + speed * 0.009).min(0.28);
                let kick = normal * ((va - vb).dot(normal).max(0.0) * 0.55 + 0.5);
                if a == 0 {
                    damage::projectile(
                        &mut f.damage,
                        &mut f.integrity,
                        qa.conjugate() * (centre - pa),
                        dose,
                    );
                    f.position -= normal * separation;
                    f.velocity -= kick;
                } else {
                    let t = &mut c.targets[a - 1];
                    damage::projectile(
                        &mut t.damage,
                        &mut t.health,
                        qa.conjugate() * (centre - pa),
                        dose,
                    );
                    t.position -= normal * separation;
                    t.velocity -= kick;
                    t.center -= normal * separation;
                }
                let t = &mut c.targets[b - 1];
                damage::projectile(
                    &mut t.damage,
                    &mut t.health,
                    qb.conjugate() * (centre - pb),
                    dose,
                );
                t.position += normal * separation;
                t.velocity += kick;
                t.center += normal * separation;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn largest_formation_checks_the_last_aircraft_pair_without_overflow() {
        let mut f = super::super::flight();
        let mut c = Combat::default();
        c.start(&f, 1);
        let template = c.targets[0].clone();
        c.targets = vec![template; 11];
        for (i, t) in c.targets.iter_mut().enumerate() {
            t.position = f.position + DVec3::X * (1000. + i as f64 * 1000.);
            t.velocity = DVec3::ZERO;
        }
        c.targets[10].position = c.targets[9].position + DVec3::X * 0.5;
        c.targets[10].velocity = DVec3::X * 50.;
        let old: Vec<_> = c
            .targets
            .iter()
            .map(|t| (t.position, t.orientation))
            .collect();
        advance(&mut c, &mut f, &old, 0.01);
        assert!(!c.targets[9].alive && !c.targets[10].alive);
        assert!(!f.crashed);
        assert_eq!(c.hits, 2);
    }
    fn encounter(speed: f64) -> (Combat, Flight, Vec<(DVec3, DQuat)>) {
        let mut f = super::super::flight();
        f.velocity = DVec3::ZERO;
        let mut c = Combat::default();
        c.start(&f, 1);
        c.targets.truncate(1);
        c.targets[0].position = f.position + DVec3::X * 0.5;
        c.targets[0].velocity = DVec3::X * speed;
        c.targets[0].orientation = f.orientation;
        let old = vec![(c.targets[0].position, c.targets[0].orientation)];
        (c, f, old)
    }
    #[test]
    fn destructive_collision_kills_both_and_counts_once() {
        let (mut c, mut f, old) = encounter(25.0);
        advance(&mut c, &mut f, &old, 0.01);
        assert!(f.crashed && !c.targets[0].alive);
        assert_eq!(c.hits, 1);
        advance(&mut c, &mut f, &old, 0.01);
        assert_eq!(c.hits, 1);
    }
    #[test]
    fn low_speed_contact_is_localized_and_debounced() {
        let (mut c, mut f, old) = encounter(3.0);
        advance(&mut c, &mut f, &old, 0.01);
        assert!(!f.crashed && c.targets[0].alive);
        assert!(f.integrity < 1.0 && c.targets[0].health < 1.0);
        let integrity = f.integrity;
        advance(&mut c, &mut f, &old, 0.01);
        assert_eq!(f.integrity, integrity);
    }
    #[test]
    fn swept_crossing_and_empty_nose_space_are_distinguished() {
        assert!(
            airframe::aircraft_contact(
                DVec3::X * -100.0,
                DVec3::X * 100.0,
                DQuat::IDENTITY,
                DQuat::IDENTITY
            )
            .is_some()
        );
        assert!(
            airframe::aircraft_contact(
                DVec3::new(15.0, 0.0, 0.0),
                DVec3::new(15.0, 0.0, 1.0),
                DQuat::IDENTITY,
                DQuat::IDENTITY
            )
            .is_none()
        );
    }
    #[test]
    fn enemy_enemy_collision_destroys_two_and_does_not_hurt_player() {
        let mut f = super::super::flight();
        let mut c = Combat::default();
        c.start(&f, 1);
        c.targets[1].position = c.targets[0].position + DVec3::X * 0.5;
        c.targets[1].orientation = c.targets[0].orientation;
        c.targets[1].velocity = c.targets[0].velocity + DVec3::X * 80.0;
        let old = c
            .targets
            .iter()
            .map(|t| (t.position, t.orientation))
            .collect::<Vec<_>>();
        advance(&mut c, &mut f, &old, 0.01);
        assert!(!f.crashed);
        assert!(!c.targets[0].alive && !c.targets[1].alive);
        assert_eq!(c.hits, 2);
        advance(&mut c, &mut f, &old, 0.01);
        assert_eq!(c.hits, 2);
    }
}
