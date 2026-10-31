//! Passive warning approximation: emitter bearings and modes, not radar ranges.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Search,
    Track,
    Missile,
}
#[derive(Clone)]
struct Contact {
    id: usize,
    direction: DVec3,
    level: Level,
    seen: f64,
}
#[derive(Default)]
pub struct Receiver {
    contacts: Vec<Contact>,
    generation: Option<u64>,
    now: f64,
}
impl Receiver {
    pub fn update(&mut self, c: &Combat, f: &Flight) {
        let now = if c.active { c.clock } else { f.elapsed };
        if self.generation != Some(c.generation) || now < self.now {
            self.contacts.clear();
        }
        self.generation = Some(c.generation);
        self.now = now;
        if !f.battery || f.damage.avionics < 0.25 || f.crashed {
            self.contacts.clear();
            return;
        }
        self.contacts.retain(|t| now - t.seen < 5.);
        for (id, position, level) in emissions(c, f) {
            let contact = Contact {
                id,
                direction: (position - f.position).normalize_or_zero(),
                level,
                seen: now,
            };
            if let Some(old) = self.contacts.iter_mut().find(|t| t.id == id) {
                *old = contact;
            } else {
                self.contacts.push(contact);
            }
        }
        self.contacts.sort_by(|a, b| {
            b.level
                .cmp(&a.level)
                .then_with(|| b.seen.total_cmp(&a.seen))
                .then(a.id.cmp(&b.id))
        });
        self.contacts.truncate(16);
    }
    pub fn fingerprint(&self) -> u64 {
        self.contacts.iter().fold(0, |hash, t| {
            hash.rotate_left(5)
                ^ t.id as u64
                ^ (t.level as u64).rotate_left(13)
                ^ t.direction.x.to_bits()
                ^ t.direction.z.to_bits()
        })
    }
}
fn emissions(c: &Combat, f: &Flight) -> Vec<(usize, DVec3, Level)> {
    let mut result = Vec::new();
    for (i, t) in c.targets.iter().enumerate() {
        if !t.alive
            || t.damage.radar < 0.25
            || t.damage.avionics < 0.2
            || c.bvr.as_ref().is_some_and(|b| i >= b.enemies)
            || (c.bvr.is_none() && t.pilot.is_none())
        {
            continue;
        }
        let offset = f.position - t.position;
        if offset.length() > 240000. {
            continue;
        }
        let local = t.orientation.conjugate() * offset;
        let az = local.x.atan2(-local.z).to_degrees();
        let el = local.y.atan2(local.x.hypot(local.z)).to_degrees();
        if az.abs() > 65. || el.abs() > 45. {
            continue;
        }
        let tracking = c.bvr.as_ref().map_or_else(
            || t.pilot.as_ref().is_some_and(|p| p.radar_tracking()),
            |b| b.rwr_tracking(c, i),
        );
        let sweep = (((c.clock * 32. + i as f64 * 27.).rem_euclid(240.) - 120.).abs() - 60.)
            .clamp(-60., 60.);
        // Terrain line of sight last: it is the only expensive gate.
        if (tracking || (az - sweep).abs() < 10.)
            && guidance::terrain_visible(t.position, f.position)
        {
            result.push((
                i,
                t.position,
                if tracking {
                    Level::Track
                } else {
                    Level::Search
                },
            ));
        }
    }
    if let Some(n)=&c.naval {for m in n.rounds.iter().filter(|m|m.alive&&m.kind==super::naval::Kind::AirToAir&&m.target==super::naval::TargetId::Player&&m.age>1.){if guidance::terrain_visible(m.position,f.position){result.push((100000+(m.id%100000)as usize,m.position,Level::Missile));}}}
    // A passive RF receiver sees a missile only after its radar seeker radiates.
    for (i, m) in c.missiles_for_draw().enumerate() {
        let hostile = c.bvr.as_ref().map_or(i >= c.missiles.len(), |b| {
            b.shots.get(i).is_some_and(|s| !s.friendly)
        });
        if !hostile || !m.alive || m.kind != Weapon::Radar || !m.gnc.active {
            continue;
        }
        let offset = f.position - m.position;
        if offset.length() > 35000. {
            continue;
        }
        if m.gnc
            .gimbal
            .normalize_or_zero()
            .dot(offset.normalize_or_zero())
            < 60_f64.to_radians().cos()
            || !guidance::terrain_visible(m.position, f.position)
        {
            continue;
        }
        result.push((1000 + i, m.position, Level::Missile));
    }
    result
}
pub fn draw(data: &mut [u8], receiver: &Receiver, f: &Flight) {
    use crate::cockpit::{radar_label as text, radar_line as line};
    let green = [95, 230, 175, 255];
    let dim = [35, 100, 85, 255];
    let amber = [255, 190, 65, 255];
    let red = [255, 80, 65, 255];
    text(data, "RWR / PASSIVE THREAT BEARING", 42, 5, green);
    if f.damage.avionics < 0.25 || f.crashed {
        text(data, "RECEIVER FAILURE", 74, 87, red);
        return;
    }
    let center = Vec2::new(128., 88.);
    for radius in [24., 40., 56.] {
        for step in 0..64 {
            let a = step as f32 * std::f32::consts::TAU / 64.;
            let b = (step + 1) as f32 * std::f32::consts::TAU / 64.;
            line(
                data,
                (center.x + radius * a.cos()) as i32,
                (center.y + radius * a.sin()) as i32,
                (center.x + radius * b.cos()) as i32,
                (center.y + radius * b.sin()) as i32,
                dim,
            );
        }
    }
    for (label, x, y) in [
        ("12", 123, 21),
        ("3", 191, 85),
        ("6", 126, 149),
        ("9", 61, 85),
    ] {
        text(data, label, x, y, green);
    }
    line(data, 128, 81, 128, 95, green);
    line(data, 121, 91, 128, 86, green);
    line(data, 128, 86, 135, 91, green);
    let mut placed: Vec<Vec2> = Vec::new();
    for contact in receiver.contacts.iter().take(8) {
        let local = f.orientation.conjugate() * contact.direction;
        let bearing = local.x.atan2(-local.z) as f32;
        let stale = receiver.now - contact.seen > 1.;
        let radius = match contact.level {
            Level::Search => 56.,
            Level::Track => 40.,
            Level::Missile => 24.,
        };
        let point = |angle: f32| center + Vec2::new(angle.sin(), -angle.cos()) * radius;
        let true_point = point(bearing);
        let mut p = true_point;
        for offset in [0., 0.25, -0.25, 0.5, -0.5, 0.75, -0.75, 1., -1.] {
            p = point(bearing + offset);
            if placed
                .iter()
                .all(|old| (old.x - p.x).abs() > 21. || (old.y - p.y).abs() > 11.)
            {
                break;
            }
        }
        placed.push(p);
        let color = if stale {
            dim
        } else {
            match contact.level {
                Level::Search => green,
                Level::Track => amber,
                Level::Missile => red,
            }
        };
        if p.distance(true_point) > 2. {
            line(
                data,
                p.x as i32,
                p.y as i32,
                true_point.x as i32,
                true_point.y as i32,
                dim,
            );
        }
        let symbol = match contact.level {
            Level::Search => "S",
            Level::Track => "L",
            Level::Missile => "M",
        };
        let label = if contact.id < 1000 {
            format!("{}{}", symbol, contact.id + 1)
        } else {
            format!("M{}", contact.id - 999)
        };
        text(data, &label, p.x as i32 - 6, p.y as i32 - 3, color);
        if contact.level == Level::Missile && (receiver.now * 4.).fract() < 0.65 {
            for y in [-6, 7] {
                line(
                    data,
                    p.x as i32 - 8,
                    p.y as i32 + y,
                    p.x as i32 + 13,
                    p.y as i32 + y,
                    red,
                );
            }
        }
    }
    let level = receiver.contacts.first().map(|t| t.level);
    text(
        data,
        match level {
            Some(Level::Missile) => "ACTIVE MISSILE / CHAFF",
            Some(Level::Track) => "RADAR TRACK / DEFEND",
            Some(Level::Search) => "RADAR SEARCH",
            None => "NO RF THREATS",
        },
        59,
        162,
        if level == Some(Level::Missile) {
            red
        } else if level == Some(Level::Track) {
            amber
        } else {
            green
        },
    );
    text(data, "S SEARCH   L TRACK   M ACTIVE", 44, 174, green);
    text(data, "BEARING ONLY / RINGS = PRIORITY", 32, 184, dim);
    text(
        data,
        &format!("{}", receiver.contacts.len()),
        211,
        22,
        green,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scene() -> (Combat, Flight) {
        let f = bvr::flight();
        let mut c = Combat::default();
        bvr::start(&mut c, &f, 1, &bvr::Menu::default());
        c.targets[0].position = f.position + f.forward() * 20000.;
        c.targets[0].orientation = facing(-f.forward(), f.up());
        c.clock = 1.875;
        (c, f)
    }
    #[test]
    fn passive_search_ignores_own_radar_and_rejects_friendlies_and_failed_emitters() {
        let (mut c, f) = scene();
        c.bvr.as_mut().unwrap().radar.powered = false;
        let search = emissions(&c, &f);
        assert!(search.iter().any(|t| t.0 == 0 && t.2 == Level::Search));
        assert!(!search.iter().any(|t| t.0 == 2));
        c.targets[0].damage.radar = 0.;
        assert!(!emissions(&c, &f).iter().any(|t| t.0 == 0));
        c.targets[0].damage.radar = 1.;
        c.targets[0].orientation = f.orientation;
        assert!(!emissions(&c, &f).iter().any(|t| t.0 == 0));
    }
    #[test]
    fn receiver_coasts_expires_resets_and_fails_with_avionics_or_power() {
        let (mut c, mut f) = scene();
        let mut r = Receiver::default();
        r.update(&c, &f);
        assert!(r.contacts.iter().any(|t| t.id == 0));
        c.targets[0].damage.radar = 0.;
        c.clock += 2.;
        r.update(&c, &f);
        assert!(r.contacts.iter().any(|t| t.id == 0));
        c.clock += 4.;
        r.update(&c, &f);
        assert!(!r.contacts.iter().any(|t| t.id == 0));
        c.targets[0].damage.radar = 1.;
        c.clock = 1.875;
        r.update(&c, &f);
        assert!(!r.contacts.is_empty());
        c.generation += 1;
        c.targets[0].damage.radar = 0.;
        r.update(&c, &f);
        assert!(!r.contacts.iter().any(|t| t.id == 0));
        f.battery = false;
        r.update(&c, &f);
        assert!(r.contacts.is_empty());
        f.battery = true;
        f.damage.avionics = 0.;
        r.update(&c, &f);
        assert!(r.contacts.is_empty());
    }
    #[test]
    fn active_radar_missile_is_detected_but_ir_and_passive_midcourse_are_not() {
        let (mut c, mut f) = scene();
        f.paused = false;
        let position = c.targets[0].position;
        let velocity = c.targets[0].velocity;
        let tr = &mut c.bvr.as_mut().unwrap().radar.tracks[0];
        *tr = bvr::Track {
            position,
            velocity,
            seen: c.clock,
            confidence: 1.,
            reports: 3,
            friendly_report: false,
        };
        c.lock = c.weapon.lock_time();
        assert!(bvr::fire(&mut c, &f));
        c.bvr.as_mut().unwrap().shots[0] = bvr::Shot {
            launcher: Some(0),
            target: None,
            friendly: false,
        };
        c.missiles[0].position = f.position + f.forward() * 6000.;
        c.missiles[0].gnc.gimbal = -f.forward();
        c.missiles[0].gnc.active = false;
        assert!(!emissions(&c, &f).iter().any(|t| t.2 == Level::Missile));
        assert!(
            emissions(&c, &f)
                .iter()
                .any(|t| t.0 == 0 && t.2 == Level::Track)
        );
        c.missiles[0].gnc.active = true;
        assert!(emissions(&c, &f).iter().any(|t| t.2 == Level::Missile));
        c.missiles[0].kind = Weapon::Heat;
        assert!(!emissions(&c, &f).iter().any(|t| t.2 == Level::Missile));
    }
    #[test]
    fn raster_shows_priority_and_aircraft_relative_bearings() {
        let (c, f) = scene();
        let mut r = Receiver::default();
        r.update(&c, &f);
        r.contacts.push(Contact {
            id: 1001,
            direction: f.orientation * DVec3::X,
            level: Level::Missile,
            seen: r.now,
        });
        r.contacts.sort_by(|a, b| b.level.cmp(&a.level));
        let mut data = vec![0; 256 * 192 * 4];
        draw(&mut data, &r, &f);
        assert!(data.chunks_exact(4).any(|p| p == [255, 80, 65, 255]));
        let mut empty = vec![0; data.len()];
        draw(&mut empty, &Receiver::default(), &f);
        assert_ne!(data, empty);
    }
}
