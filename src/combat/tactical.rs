//! Shared tactical picture. Rendering consumes sensor reports rather than truth;
//! the explicit difficulty aid may reveal all contacts without changing seekers.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Aircraft,
    Ship,
    Weapon,
}
#[derive(Clone, Debug)]
pub struct Contact {
    pub position: DVec3,
    pub friendly: bool,
    pub category: Category,
    pub aircraft: Option<usize>,
    pub label: String,
    pub freshness: f32,
}
pub fn contacts(c: &Combat, f: &Flight, reveal: bool) -> Vec<Contact> {
    let mut out = Vec::with_capacity(128);
    for (i, t) in c
        .targets
        .iter()
        .enumerate()
        .filter(|(_, t)| c.active && t.alive)
    {
        let friend = c.bvr.as_ref().is_some_and(|b| b.friendly(i));
        let radar = c.bvr.as_ref().map_or(&c.radar, |b| &b.radar);
        let local = radar
            .tracks
            .get(i)
            .filter(|tr| tr.valid(c.clock))
            .map(|tr| {
                (
                    tr.position + tr.velocity * (c.clock - tr.seen),
                    c.clock - tr.seen,
                )
            });
        let fleet = c.naval.as_ref().and_then(|n| {
            n.track_for(i).map(|tr| {
                (
                    tr.position + tr.velocity * (n.clock - tr.seen),
                    n.clock - tr.seen,
                )
            })
        });
        let estimate = if reveal || friend {
            Some((t.position, 0.))
        } else {
            match (local, fleet) {
                (Some(a), Some(b)) => Some(if a.1 <= b.1 { a } else { b }),
                (a, b) => a.or(b),
            }
        };
        if let Some((p, age)) = estimate {
            out.push(Contact {
                position: p,
                friendly: friend,
                category: Category::Aircraft,
                aircraft: Some(i),
                label: format!(
                    "{} {:02}{}",
                    if friend { "ALLY" } else { "CONTACT" },
                    i + 1,
                    if age > 3. { " COAST" } else { "" }
                ),
                freshness: (1. - age / 10.).clamp(0.2, 1.) as f32,
            });
        }
    }
    if f.carrier.active {
        for ship in 0..3 {
            if c.naval.as_ref().is_none_or(|n| n.ships[ship].hull > 0.) {
                out.push(Contact {
                    position: crate::carrier::ship_origin(f, ship, f.elapsed),
                    friendly: true,
                    category: Category::Ship,
                    aircraft: None,
                    label: ["CARRIER", "DDG 125", "DDG 128"][ship].into(),
                    freshness: 1.,
                });
            }
        }
    }
    for (i, m) in c
        .missiles_for_draw()
        .enumerate()
        .filter(|(_, m)| c.active && m.alive)
    {
        let own = c.bvr.as_ref().map_or(i < c.missiles.len(), |b| {
            b.shots.get(i).is_some_and(|s| s.friendly)
        });
        let r = m.position - f.position;
        let warning = r.length() < 35000.
            && (m.velocity - f.velocity).dot(-r) > 0.
            && guidance::terrain_visible(f.position, m.position);
        if reveal || own || warning {
            out.push(Contact {
                position: m.position,
                friendly: own,
                category: Category::Weapon,
                aircraft: None,
                label: if own {
                    "FRIEND MISSILE"
                } else {
                    "HOSTILE MISSILE"
                }
                .into(),
                freshness: 1.,
            });
        }
    }
    if let Some(n) = c.naval.as_ref().filter(|_| c.active) {
        for m in n.rounds.iter().filter(|m| m.alive) {
            let friend = matches!(m.source, naval::TargetId::Ship(_) | naval::TargetId::Player);
            let track = n
                .missile_tracks
                .iter()
                .find(|tr| tr.id == m.id && tr.valid(n.clock));
            let r = m.position - f.position;
            let warning = r.length() < 25000.
                && (m.velocity - f.velocity).dot(-r) > 0.
                && guidance::terrain_visible(f.position, m.position);
            if reveal || friend || track.is_some() || warning {
                let (p, age) = if !reveal && !friend {
                    track.map_or((m.position, 0.), |t| {
                        (
                            t.position + t.velocity * (n.clock - t.seen),
                            n.clock - t.seen,
                        )
                    })
                } else {
                    (m.position, 0.)
                };
                out.push(Contact {
                    position: p,
                    friendly: friend,
                    category: Category::Weapon,
                    aircraft: None,
                    label: if friend {
                        "FLEET MISSILE"
                    } else {
                        "HOSTILE MISSILE"
                    }
                    .into(),
                    freshness: (1. - age / 8.).max(0.2) as f32,
                });
            }
        }
    }
    out
}
pub fn color(c: &Contact) -> Color {
    if c.friendly {
        Color::srgba(0.35, 0.85, 1., c.freshness)
    } else {
        Color::srgba(1., 0.35, 0.2, c.freshness)
    }
}
pub fn draw_scope(data: &mut [u8], c: &Combat, f: &Flight, range: f64) {
    use crate::cockpit::{radar_label as text, radar_line as line};
    let dim = [25, 85, 72, 255];
    text(
        data,
        if c.map_reveal {
            "TACTICAL / ALL CONTACTS"
        } else {
            "TACTICAL / SHARED SENSORS"
        },
        20,
        3,
        [90, 245, 175, 255],
    );
    text(data, &format!("{:.0}KM H-UP", range / 1000.), 20, 15, dim);
    if !f.battery {
        text(data, "NO ELECTRICAL POWER", 35, 90, [255, 185, 75, 255]);
        return;
    }
    for x in [40, 84, 128, 172, 216] {
        line(data, x, 40, x, 156, dim);
    }
    for y in [40, 69, 98, 127, 156] {
        line(data, 40, y, 216, y, dim);
    }
    let up = f.up();
    let ahead = (f.forward() - up * f.forward().dot(up)).normalize_or_zero();
    let right = ahead.cross(up);
    for mark in contacts(c, f, c.map_reveal) {
        let r = mark.position - f.position;
        let x = 128 + (r.dot(right) / range * 88.) as i32;
        let y = 100 - (r.dot(ahead) / range * 56.) as i32;
        if !(43..213).contains(&x) || !(43..153).contains(&y) {
            continue;
        }
        let selected = mark.aircraft == Some(c.selected) && !mark.friendly;
        let color = if selected {
            [255, 185, 75, (255. * mark.freshness) as u8]
        } else if mark.friendly {
            [90, 185, 255, (255. * mark.freshness) as u8]
        } else {
            [255, 110, 65, (255. * mark.freshness) as u8]
        };
        match mark.category {
            Category::Aircraft => {
                line(data, x - 3, y, x, y - 3, color);
                line(data, x, y - 3, x + 3, y, color);
                line(data, x + 3, y, x, y + 3, color);
                line(data, x, y + 3, x - 3, y, color);
            }
            Category::Ship => {
                line(data, x - 4, y - 2, x + 4, y - 2, color);
                line(data, x + 4, y - 2, x + 3, y + 3, color);
                line(data, x + 3, y + 3, x - 3, y + 3, color);
                line(data, x - 3, y + 3, x - 4, y - 2, color);
            }
            Category::Weapon => {
                line(data, x, y - 3, x - 2, y + 2, color);
                line(data, x, y - 3, x + 2, y + 2, color);
            }
        }
    }
    line(data, 122, 100, 134, 100, [90, 245, 175, 255]);
    line(data, 128, 95, 128, 105, [90, 245, 175, 255]);
    text(data, "BLUE FRIEND / ORANGE CONTACT", 20, 173, dim);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_ship_tracks_reveal_only_confirmed_contacts_and_expire_without_truth_leaks() {
        let f = naval::flight(false);
        let mut c = Combat::default();
        naval::start(&mut c, &f, 1, 1);
        assert_eq!(
            contacts(&c, &f, false)
                .iter()
                .filter(|m| m.category == Category::Aircraft)
                .count(),
            0
        );
        assert_eq!(
            contacts(&c, &f, true)
                .iter()
                .filter(|m| m.category == Category::Aircraft)
                .count(),
            4
        );
        let n = c.naval.as_mut().unwrap();
        let r = &n.raiders[2];
        let p = c.targets[r.slot].position;
        n.tracks.push(naval::Track {
            id: r.id,
            position: p,
            velocity: DVec3::ZERO,
            seen: 0.,
            hits: 2,
            role: None,
        });
        c.targets[2].position += DVec3::X * 1000.;
        let marks = contacts(&c, &f, false);
        let a = marks.iter().find(|m| m.aircraft == Some(2)).unwrap();
        assert_eq!(a.position, p);
        assert_eq!(
            marks
                .iter()
                .filter(|m| m.category == Category::Ship)
                .count(),
            3
        );
        c.naval.as_mut().unwrap().clock = 9.;
        assert!(
            !contacts(&c, &f, false)
                .iter()
                .any(|m| m.category == Category::Aircraft)
        );
    }
    #[test]
    fn practice_map_shows_own_radar_tracks_and_hides_unsensed_aircraft() {
        let mut f = super::super::flight();
        f.paused = false;
        let mut c = Combat::default();
        c.start(&f, 1);
        assert!(
            !contacts(&c, &f, false)
                .iter()
                .any(|m| m.category == Category::Aircraft)
        );
        for _ in 0..360 {
            c.advance(&f, 1. / 120.);
        }
        assert!(
            contacts(&c, &f, false)
                .iter()
                .any(|m| m.category == Category::Aircraft)
        );
        c.active = false;
        assert!(contacts(&c, &f, true).is_empty());
    }
}
