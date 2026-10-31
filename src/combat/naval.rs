//! Bounded, deterministic carrier defense. All tactical performance is an estimate.
use super::*;
use crate::carrier;
pub mod damage;
mod radio;
mod visual;
mod weapons;
pub use radio::Speech;
pub use visual::{draw, setup};
pub const AIRCRAFT_LIMIT: usize = 12;
pub const WEAPON_LIMIT: usize = 96;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Fighter,
    Bomber,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetId {
    Aircraft(u64),
    Missile(u64),
    Ship(usize),
    Player,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Standard,
    Essm,
    AntiShip,
    AirToAir,
}
#[derive(Clone, Debug)]
pub struct Round {
    pub id: u64,
    pub source: TargetId,
    pub target: TargetId,
    pub kind: Kind,
    pub position: DVec3,
    pub previous: DVec3,
    pub velocity: DVec3,
    pub age: f64,
    pub alive: bool,
    pub support_lost: f64,
}
#[derive(Clone, Default, Debug)]
pub struct Track {
    pub id: u64,
    pub position: DVec3,
    pub velocity: DVec3,
    pub seen: f64,
    pub hits: u32,
    pub role: Option<Role>,
}
impl Track {
    pub fn valid(&self, now: f64) -> bool {
        self.hits >= 2 && now - self.seen < 8.
    }
}
#[derive(Clone, Debug)]
pub struct Raider {
    pub id: u64,
    pub slot: usize,
    pub role: Role,
    pub stores: u8,
    pub aam: u8,
    pub next_shot: f64,
    pub withdrew: bool,
    pub raid: u32,
}
#[derive(Clone, Debug)]
pub struct Burst {
    pub ship: usize,
    pub target: TargetId,
    pub position: DVec3,
    pub previous: DVec3,
    pub velocity: DVec3,
    pub age: f64,
    pub rounds: u16,
    pub alive: bool,
}
#[derive(Clone, Debug)]
pub struct Flash {
    pub position: DVec3,
    pub age: f64,
    pub water: bool,
}
#[derive(Clone, Debug)]
pub struct State {
    pub difficulty: usize,
    pub ships: [damage::Ship; 3],
    pub raiders: Vec<Raider>,
    pub tracks: Vec<Track>,
    pub missile_tracks: Vec<Track>,
    pub rounds: Vec<Round>,
    pub bursts: Vec<Burst>,
    pub flashes: Vec<Flash>,
    pub clock: f64,
    pub next_wave: f64,
    pub waves: u32,
    pub defeated: u32,
    pub ship_kills: u32,
    pub player_kills: u32,
    pub interceptions: u32,
    pub ended: bool,
    pub survival: f64,
    pub radio_open: bool,
    pub report: String,
    pub report_until: f64,
    pub report_serial: u64,
    pub seed: u64,
    active_raids: [Option<u32>; 12],
    weapon_budget: usize,
    next_id: u64,
    next_scan: f64,
    next_engage: f64,
    next_damage: f64,
    pub loadout_tanks: bool,
}
impl State {
    pub fn new(difficulty: usize, seed: u64, tanks: bool) -> Self {
        Self {
            difficulty: difficulty.min(2),
            ships: std::array::from_fn(damage::Ship::new),
            raiders: Vec::with_capacity(12),
            tracks: Vec::with_capacity(12),
            missile_tracks: Vec::with_capacity(96),
            rounds: Vec::with_capacity(WEAPON_LIMIT),
            bursts: Vec::with_capacity(96),
            flashes: Vec::with_capacity(32),
            clock: 0.,
            next_wave: 0.,
            waves: 0,
            defeated: 0,
            ship_kills: 0,
            player_kills: 0,
            interceptions: 0,
            ended: false,
            survival: 0.,
            radio_open: false,
            report: String::new(),
            report_until: 0.,
            report_serial: 0,
            seed,
            active_raids: [None; 12],
            weapon_budget: WEAPON_LIMIT,
            next_id: 1,
            next_scan: 0.,
            next_engage: 0.,
            next_damage: 0.1,
            loadout_tanks: tanks,
        }
    }
    fn random(&mut self) -> f64 {
        self.seed = self
            .seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.seed >> 11) as f64 / ((1u64 << 53) as f64)
    }
    fn id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    pub fn track_for(&self, slot: usize) -> Option<&Track> {
        let id = self.raiders.iter().find(|r| r.slot == slot)?.id;
        self.tracks
            .iter()
            .find(|t| t.id == id && t.valid(self.clock))
    }
    pub fn deck_usable(&self) -> bool {
        !self.ended && self.ships[0].deck_usable()
    }
    pub fn spawn(&mut self, c: &mut Combat, f: &Flight) {
        if self.ended || self.clock < self.next_wave || self.raiders.len() + 4 > AIRCRAFT_LIMIT {
            return;
        }
        self.waves += 1;
        if let Some(slot) = self.active_raids.iter_mut().find(|r| r.is_none()) {
            *slot = Some(self.waves);
        }
        let angle = self.random() * std::f64::consts::TAU;
        let radial = DVec3::new(angle.sin(), 0., angle.cos());
        let tangent = DVec3::new(radial.z, 0., -radial.x);
        for k in 0..4 {
            let role = if k < 2 { Role::Fighter } else { Role::Bomber };
            let height = if role == Role::Bomber { 3800. } else { 5000. };
            let p = carrier::ship_world(
                f,
                0,
                radial * (42000. + k as f64 * 1500.)
                    + tangent * (k as f64 - 1.5) * 1200.
                    + DVec3::Y * height,
                f.elapsed,
            );
            let forward = (carrier::ship_origin(f, 0, f.elapsed) - p).normalize();
            let id = self.id();
            let slot = (0..c.targets.len())
                .find(|&i| {
                    !c.targets[i].alive
                        && !self.raiders.iter().any(|r| r.slot == i)
                        && !c.missiles.iter().any(|m| m.alive && m.target == i)
                })
                .unwrap_or(c.targets.len());
            if slot >= AIRCRAFT_LIMIT {
                break;
            }
            let mut t = Target {
                dynamics: ai::airframe(),
                brain: ai::Brain {
                    id: slot,
                    ..default()
                },
                damage: AircraftDamage {
                    rng: id.wrapping_mul(131) + 7,
                    ..default()
                },
                wreck_age: 0.,
                throttle: 0.75,
                fuel: 6000.,
                health: if role == Role::Bomber { 2. } else { 1. },
                pilot: None,
                position: p,
                velocity: forward * 240.,
                orientation: facing(forward, p.normalize()),
                alive: true,
                center: p,
                right: tangent,
                forward,
                phase: 0.,
                altitude: p.length() - RADIUS,
                speed: 240.,
            };
            ai::initialize(&mut t);
            crate::aircraft::select(&mut t.dynamics, crate::aircraft::Aircraft::Su27);
            if slot == c.targets.len() {
                c.targets.push(t);
            } else {
                c.targets[slot] = t;
            }
            c.fx.reset_actor(slot+1);
            self.raiders.push(Raider {
                id,
                slot,
                role,
                stores: if role == Role::Bomber { 2 } else { 1 },
                aam: if role == Role::Fighter { 4 } else { 0 },
                next_shot: self.clock + 10.,
                withdrew: false,
                raid: self.waves,
            });
        }
        self.next_wave = self.clock + [300., 240., 180.][self.difficulty];
    }
    pub fn pilots(&mut self, c: &mut Combat, f: &Flight, dt: f64) {
        for t in c.targets.iter_mut().filter(|t|!t.alive){t.wreck_age+=dt;}
        for r in &mut self.raiders {
            let Some(t) = c.targets.get_mut(r.slot).filter(|t| t.alive) else {
                continue;
            };
            let ship = carrier::ship_origin(f, 0, f.elapsed);
            let outward = (t.position - ship).normalize();
            let player = f.position - t.position;
            let hunt = r.role == Role::Fighter
                && !f.crashed
                && !f.ejection.active
                && player.length() < 18000.;
            let destination = if r.withdrew {
                t.position + outward * 20000.
            } else if hunt {
                f.position + f.velocity * 3.
            } else {
                ship + t.position.normalize() * if r.role == Role::Bomber { 3500. } else { 4200. }
            };
            ai::fly(
                t,
                &c.environment,
                (destination - t.position).normalize_or(t.forward),
                if r.role == Role::Bomber { 230. } else { 270. },
                match self.difficulty {
                    0 => dogfight::Difficulty::Training,
                    1 => dogfight::Difficulty::Balanced,
                    _ => dogfight::Difficulty::Hard,
                },
                dt,
            );
            if r.withdrew && t.position.distance(ship) > 65000. {
                t.alive = false;
            }
        }
    }
    fn scan(&mut self, c: &Combat, f: &Flight) {
        if self.clock < self.next_scan {
            return;
        }
        self.next_scan = self.clock + 1.;
        for r in &self.raiders {
            let Some(t) = c.targets.get(r.slot).filter(|t| t.alive) else {
                continue;
            };
            let detected = (0..3).any(|ship| {
                self.detect(
                    f,
                    ship,
                    t.position,
                    if r.role == Role::Bomber { 12. } else { 3. },
                )
            });
            if detected {
                if let Some(track) = self.tracks.iter_mut().find(|t| t.id == r.id) {
                    track.position = t.position;
                    track.velocity = t.velocity;
                    track.seen = self.clock;
                    track.hits += 1;
                    if track.hits >= 4 {
                        track.role = Some(r.role);
                    }
                } else {
                    self.tracks.push(Track {
                        id: r.id,
                        position: t.position,
                        velocity: t.velocity,
                        seen: self.clock,
                        hits: 1,
                        role: None,
                    });
                }
            }
        }
        for m in self
            .rounds
            .iter()
            .filter(|m| m.alive && m.kind == Kind::AntiShip)
        {
            if (0..3).any(|ship| self.detect(f, ship, m.position, 0.1)) {
                if let Some(t) = self.missile_tracks.iter_mut().find(|t| t.id == m.id) {
                    t.position = m.position;
                    t.velocity = m.velocity;
                    t.seen = self.clock;
                    t.hits += 1;
                } else {
                    self.missile_tracks.push(Track {
                        id: m.id,
                        position: m.position,
                        velocity: m.velocity,
                        seen: self.clock,
                        hits: 1,
                        role: None,
                    });
                }
            }
        }
        self.missile_tracks.retain(|t| {
            self.clock - t.seen < 3. && self.rounds.iter().any(|m| m.id == t.id && m.alive)
        });
        self.tracks.retain(|t| {
            self.clock - t.seen < 8.
                && self
                    .raiders
                    .iter()
                    .any(|r| r.id == t.id && c.targets[r.slot].alive)
        });
    }
    pub fn detect(&self, f: &Flight, ship: usize, p: DVec3, signature: f64) -> bool {
        let s = &self.ships[ship];
        let authority = s.radar * s.power;
        if authority < 0.2 || s.sunk {
            return false;
        }
        let antenna = carrier::ship_world(
            f,
            ship,
            DVec3::new(0., if ship == 0 { 55. } else { 38. }, 0.),
            f.elapsed,
        );
        let range = antenna.distance(p);
        let h1 = (antenna.length() - RADIUS).max(0.);
        let h2 = (p.length() - RADIUS).max(0.);
        let horizon = (2. * RADIUS * h1).sqrt() + (2. * RADIUS * h2).sqrt();
        range < horizon
            && range < 140000. * signature.powf(0.25) * authority.sqrt()
            && guidance::terrain_visible(antenna, p)
            && !carrier::collider::sweep(
                f,
                antenna + (p - antenna).normalize_or_zero() * 100.,
                p,
                0.,
                0.1,
            )
            .is_some_and(|h| h.fraction < 0.99)
    }
    pub fn report(&mut self, f: &Flight) {
        self.report.clear();
        self.report.push_str("AEGIS / RADAR SWEEP\n");
        let mut count = 0;
        for t in self.tracks.iter().filter(|t| t.valid(self.clock)) {
            let (bearing, range) = bearing_range(f, t.position);
            self.report.push_str(&format!(
                "CONTACT {} / {} / {:03.0} TRUE / {:.1} NM / {:.0} FT\n",
                t.id,
                match t.role {
                    Some(Role::Fighter) => "FIGHTER",
                    Some(Role::Bomber) => "BOMBER",
                    None => "UNKNOWN",
                },
                bearing,
                range,
                (t.position.length() - RADIUS).max(0.) * 3.28084
            ));
            count += 1;
        }
        if count == 0 {
            self.report.push_str("NO CONFIRMED AIR CONTACTS");
        }
        self.report_until = self.clock + 30.;
        self.report_serial += 1;
    }
    pub fn advance(&mut self, c: &mut Combat, f: &mut Flight, dt: f64) {
        self.clock += dt;
        if !self.ended {
            self.survival = self.clock;
        }
        self.spawn(c, f);
        self.scan(c, f);
        self.weapon_budget =
            WEAPON_LIMIT.saturating_sub(c.missiles.iter().filter(|m| m.alive).count());
        if !self.ended {
            self.weapons(c, f, dt);
        }
        while self.clock >= self.next_damage {
            let step = 0.1;
            self.next_damage += step;
            for ship in &mut self.ships {
                ship.advance(step);
            }
        }
        for i in 0..3 {
            let s = &self.ships[i];
            let motion = &mut f.carrier.motion[i];
            motion.offset.z += (1. - s.propulsion) * carrier::SPEED * dt;
            motion.offset.y = -s.sink;
            let target = DQuat::from_rotation_z(s.list) * DQuat::from_rotation_x(s.trim);
            let next = motion.rotation.slerp(target, (dt / 0.1).min(1.));
            motion.angular = (motion.rotation.conjugate() * next).to_scaled_axis() / dt.max(1e-6);
            motion.rotation = next;
            motion.linear = DVec3::new(0., -s.sink_rate, (1. - s.propulsion) * carrier::SPEED);
            motion.at = f.elapsed;
        }
        f.carrier.operational = self.deck_usable();
        if !self.deck_usable() {
            f.carrier.attached = false;
            f.carrier.launch = None;
            f.carrier.arrested = false;
        }
        if self.ships[0].sunk {
            self.ended = true;
            self.report = "CARRIER LOST / DEFENSE ENDED".into();
            self.report_until = f64::INFINITY;
        }
        if self.ended{for m in &mut self.rounds{m.alive=false;}for b in &mut self.bursts{b.alive=false;}}
        for x in &mut self.flashes {
            x.age += dt;
        }
        self.flashes.retain(|x| x.age < 3.);
        for raid in &mut self.active_raids {
            if raid.is_some_and(|id| {
                !self
                    .raiders
                    .iter()
                    .any(|r| r.raid == id && c.targets[r.slot].alive)
            }) {
                self.defeated += 1;
                *raid = None;
            }
        }
        self.raiders.retain(|r| {
            c.targets[r.slot].alive || c.missiles.iter().any(|m| m.alive && m.target == r.slot)
        });
    }
    pub fn service(&mut self, c: &mut Combat, f: &mut Flight) -> bool {
        if !self.deck_usable()
            || !f.landed
            || f.crashed
            || f.ejection.active
            || f.throttle > 0.05
            || f.carrier.attached
            || f.carrier.arrested
            || f.carrier.launch.is_some()
            || !carrier::inside(carrier::ship_relative(f, 0, f.position, f.elapsed))
            || (f.velocity - carrier::ship_velocity(f, 0, f.position, f.elapsed)).length() > 1.
        {
            return false;
        }
        f.fuel = crate::aircraft::Aircraft::Hornet.spec().fuel;
        c.ammo = f.aircraft.spec().ammo;
        if let Some(fight) = &mut c.fight {
            fight.gun = f.aircraft.spec().gun;
            fight.flares = 24;
            fight.chaff = 24;
        }
        f.systems.stores.fill(true);
        if self.loadout_tanks {
            crate::aircraft::hornet::stores::fit(f);
        }
        true
    }
}
pub fn bearing_range(f: &Flight, p: DVec3) -> (f64, f64) {
    let up = f.up();
    let (north, east) = geographic_axes(up);
    let d = p - f.position;
    (
        (d.dot(east).atan2(d.dot(north)).to_degrees()).rem_euclid(360.),
        d.length() / 1852.,
    )
}
pub fn start(c: &mut Combat, f: &Flight, generation: u64, difficulty: usize) {
    *c = Combat {
        active: true,
        generation,
        ammo: f.aircraft.spec().ammo,
        ..default()
    };
    c.install_naval_weapons(f);
    if let Some(fight)=&mut c.fight{fight.difficulty=match difficulty{0=>dogfight::Difficulty::Training,1=>dogfight::Difficulty::Balanced,_=>dogfight::Difficulty::Hard};}
    let mut n = State::new(
        difficulty,
        generation.wrapping_add(77),
        f.hornet.tanks.loadout,
    );
    n.spawn(c, f);
    c.naval = Some(n);
}
pub fn flight(deck: bool) -> Flight {
    let mut f = carrier::scenario(deck);
    f.carrier.defense = true;
    if !deck {
        f.position = carrier::world(DVec3::new(0., 4500., 9000.), 0.);
        f.gear = false;
        f.gear_extension = 0.;
        f.gear_door = 0.;
        f.flaps = 0.;
        f.systems.hook_down = false;
        f.hornet.hook_angle = 0.;
        f.velocity = carrier::rotation(0.) * DVec3::NEG_Z * 220.;
        f.orientation = carrier::rotation(0.) * DQuat::from_rotation_x(0.04);
        if let Some(trim) = crate::aircraft::hornet::trim::solve(&f, 4500., 220., 0., 0.) {
            crate::aircraft::hornet::trim::apply(&mut f, trim);
            f.orientation = carrier::rotation(0.) * DQuat::from_rotation_x(trim.alpha);
        }
    }
    f.hornet.hud.mode = crate::cockpit::hornet_hud::MasterMode::Air;
    f.status = "CARRIER DEFENSE / BACKSLASH RADIO / 7 SHIP VIEW".into();
    f
}
/// Radio input is ordered before camera controls. Clears consumed key edges.
pub fn radio_controls(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut c: ResMut<Combat>,
    mut f: ResMut<Flight>,
    mut view: ResMut<ViewState>,
    mut speech: ResMut<Speech>,
    audio: Option<Res<crate::sound::Settings>>,
) {
    if (13..=18).contains(&view.scenario) && c.generation != view.generation {
        start(
            &mut c,
            &f,
            view.generation,
            ((view.scenario - 13) / 2) as usize,
        );
    }
    let Some(mut n) = c.naval.take() else {
        speech.update(view.generation, None);
        return;
    };
    if keys.just_pressed(KeyCode::Backslash) {
        n.radio_open = !n.radio_open;
        keys.clear_just_pressed(KeyCode::Backslash);
    }
    if n.radio_open {
        if keys.just_pressed(KeyCode::Escape) {
            n.radio_open = false;
            keys.clear_just_pressed(KeyCode::Escape);
        }
        if keys.just_pressed(KeyCode::Digit6) {
            n.report(&f);
            n.radio_open = false;
            keys.clear_just_pressed(KeyCode::Digit6);
        }
        if keys.just_pressed(KeyCode::Digit1) {
            let serviced = n.service(&mut c, &mut f);
            n.report = if serviced {
                "SERVICED / FUEL AND WEAPONS RESTORED"
            } else {
                "SERVICE: STOP ON DECK / IDLE / HOOK UP / RELEASE CATAPULT"
            }
            .into();
            n.report_until = n.clock + 10.;
            keys.clear_just_pressed(KeyCode::Digit1);
            n.radio_open = false;
        }
        if keys.just_pressed(KeyCode::Digit2) && (f.crashed || f.ejection.active) && n.deck_usable()
        {
            let time = f.elapsed;
            let motion = f.carrier.motion;
            let mut fresh = flight(true);
            fresh.elapsed = time;
            fresh.carrier.motion = motion;
            let p = fresh.carrier.deck_position;
            fresh.position = carrier::ship_world(
                &fresh,
                0,
                p + DVec3::Y * (crate::collision::wheel_clearance(&fresh) - 0.12),
                time,
            );
            fresh.orientation = carrier::ship_rotation(&fresh, 0, time);
            fresh.velocity = carrier::ship_velocity(&fresh, 0, fresh.position, time);
            if n.loadout_tanks {
                crate::aircraft::hornet::stores::fit(&mut fresh);
            }
            *f = fresh;
            c.ammo = f.aircraft.spec().ammo;
            c.lock = 0.;
            c.pending_main = None;
            c.pending_side = None;
            view.restore_camera();
            n.radio_open = false;
            keys.clear_just_pressed(KeyCode::Digit2);
        }
        if keys.just_pressed(KeyCode::Digit3) {
            n.ended = true;
            n.report = "DEFENSE RUN ENDED".into();
            n.report_until = f64::INFINITY;
            n.radio_open = false;
            keys.clear_just_pressed(KeyCode::Digit3);
        }
    }
    speech.update(
        view.generation,
        if audio
            .as_ref()
            .is_some_and(|s| s.menu_muted() || s.menu_level() <= 0.)
            || n.clock >= n.report_until
        {
            None
        } else {
            Some(&n)
        },
    );
    c.naval = Some(n);
}
#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (Flight, Combat) {
        let f = flight(false);
        let mut c = Combat::default();
        start(&mut c, &f, 1, 1);
        (f, c)
    }
    #[test]
    fn mission_starts_and_caps_deterministic_raids() {
        let (f, mut c) = setup();
        let mut n = c.naval.take().unwrap();
        assert_eq!(
            n.raiders.iter().filter(|r| r.role == Role::Fighter).count(),
            2
        );
        assert_eq!(
            n.raiders.iter().filter(|r| r.role == Role::Bomber).count(),
            2
        );
        for k in 1..5 {
            n.clock = k as f64 * 241.;
            n.spawn(&mut c, &f);
        }
        assert_eq!(n.raiders.len(), 12);
        assert_eq!(n.waves, 3);
        assert_eq!(c.targets.len(), 12);
        let (f2, c2) = setup();
        assert!(f.position.distance(f2.position) < 1e-8);
        assert_eq!(c.targets[0].position, c2.targets[0].position);
    }
    #[test]
    fn horizon_and_tracks_do_not_reveal_undetected_contacts() {
        let (f, mut c) = setup();
        let mut n = c.naval.take().unwrap();
        assert!(!n.detect(
            &f,
            1,
            carrier::world(DVec3::new(-500., 1., 120000.), 0.),
            1.
        ));
        n.clock = 0.;
        n.scan(&c, &f);
        assert!(n.tracks.iter().all(|t| !t.valid(n.clock)));
        n.clock = 1.;
        n.scan(&c, &f);
        assert!(!n.tracks.is_empty());
        assert!(n.tracks.iter().any(|t| t.valid(n.clock)));
        assert!(n.tracks.iter().all(|t| t.role.is_none()));
        for i in 2..5 {
            n.clock = i as f64;
            n.scan(&c, &f);
        }
        assert!(n.tracks.iter().any(|t| t.role.is_some()));
        for s in &mut n.ships {
            s.radar = 0.;
        }
        n.clock = 15.;
        n.scan(&c, &f);
        assert!(n.tracks.is_empty());
    }
    #[test]
    fn reports_are_player_relative_and_use_no_omniscient_targets() {
        let (f, mut c) = setup();
        let mut n = c.naval.take().unwrap();
        n.report(&f);
        assert!(n.report.contains("NO CONFIRMED"));
        let p = f.position + f.up() * 100. + geographic_axes(f.up()).1 * 1852.;
        let (b, r) = bearing_range(&f, p);
        assert!((b - 90.).abs() < 1.);
        assert!((r - 1.).abs() < 0.01);
        n.tracks.push(Track {
            id: 1,
            position: p,
            seen: 0.,
            hits: 4,
            role: Some(Role::Bomber),
            ..default()
        });
        n.report(&f);
        assert!(n.report.contains("BOMBER") && n.report.contains("TRUE"));
    }
    #[test]
    fn service_rejects_airborne_attached_and_damaged_deck() {
        let (mut f, mut c) = setup();
        let mut n = c.naval.take().unwrap();
        assert!(!n.service(&mut c, &mut f));
        f = flight(true);
        assert!(!n.service(&mut c, &mut f));
        f.carrier.attached = false;
        f.velocity = carrier::ship_velocity(&f, 0, f.position, f.elapsed);
        f.landed = true;
        f.throttle = 0.;
        assert!(n.service(&mut c, &mut f));
        n.ships[0].list = 0.1;
        assert!(!n.service(&mut c, &mut f));
    }
    #[test]
    fn ship_camera_cycles_and_restores_exact_view() {
        let mut v = ViewState::default();
        v.index = 3;
        v.pitch = -0.21;
        let old = (v.index, v.yaw, v.pitch, v.zoom, v.map);
        for i in 0..3 {
            v.cycle_ship();
            assert_eq!(v.ship_follow, Some(i));
            assert!(v.following());
        }
        v.cycle_ship();
        assert_eq!(v.ship_follow, None);
        assert_eq!(old, (v.index, v.yaw, v.pitch, v.zoom, v.map));
    }
    #[test]
    fn damaged_ship_frame_and_collider_move_together() {
        let (mut f, _) = setup();
        f.carrier.motion[1].offset = DVec3::new(20., -3., 10.);
        f.carrier.motion[1].rotation = DQuat::from_rotation_z(0.15);
        let p = DVec3::new(0., 4., 0.);
        let w = carrier::ship_world(&f, 1, p, 0.);
        assert!(carrier::ship_relative(&f, 1, w, 0.).distance(p) < 1e-7);
        let a = carrier::ship_world(&f, 1, DVec3::new(-35., 4., 0.), 0.);
        let b = carrier::ship_world(&f, 1, DVec3::new(35., 4., 0.), 0.);
        let hit = carrier::collider::sweep(&f, a, b, 0., 0.1).unwrap();
        assert_eq!(hit.ship, 1);
    }
    #[test]
    fn generation_reset_clears_battle_and_damage() {
        let (mut f, mut c) = setup();
        c.naval.as_mut().unwrap().ships[0].hull = 0.1;
        let mut v = ViewState::default();
        crate::reset(&mut f, 16, &mut v);
        start(&mut c, &f, v.generation, 1);
        assert_eq!(c.naval.as_ref().unwrap().ships[0].hull, 1.);
        assert_eq!(c.targets.len(), 4);
        assert_eq!(f.aircraft, crate::aircraft::Aircraft::Hornet);
    }
    #[test]
    fn fixed_step_live_wave_has_finite_bounded_state() {
        let (mut f, mut c) = setup();
        f.paused = false;
        for _ in 0..1200 {
            f.elapsed += 1. / 120.;
            c.tick(&mut f, 1. / 120.);
        }
        let n = c.naval.as_ref().unwrap();
        assert!(n.clock > 9.9);
        assert!(
            c.targets
                .iter()
                .all(|t| t.position.is_finite() && t.velocity.is_finite())
        );
        assert!(n.rounds.len() <= 96 && n.tracks.len() <= 12);
        assert!(n.tracks.iter().any(|t| t.valid(n.clock)));
    }
}
/// Deterministic headless end-to-end exercise, separate from rendering benchmarks.
pub fn study() {
    let seconds = std::env::var("NORD_NAVAL_STUDY_SECONDS")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(660.);
    let mut rows = vec![];
    let preset=std::env::var("NORD_NAVAL_STUDY_PRESET").ok().and_then(|s|s.parse::<usize>().ok());
    for difficulty in (0..3).filter(|d|preset.is_none_or(|p|p==*d)) {
        let mut f = flight(true);
        f.paused = false;
        let e = Environment {
            wind_enabled: false,
            ..default()
        };
        let mut c = Combat::default();
        start(&mut c, &f, 1, difficulty);
        let mut samples = Vec::new();
        let mut max_rounds = 0;
        let mut max_aircraft = 0;
        for k in 0..(seconds * 120.) as usize {
            let at = std::time::Instant::now();
            crate::physics::simulate(&mut f, &e, 1. / 120.);
            c.tick(&mut f, 1. / 120.);
            samples.push(at.elapsed().as_secs_f64() * 1000.);
            let n = c.naval.as_ref().unwrap();
            max_rounds = max_rounds.max(n.rounds.iter().filter(|m| m.alive).count());
            max_aircraft = max_aircraft.max(n.raiders.len());
            assert!(c.targets.iter().all(|t| t.position.is_finite()));
            if k % 12000 == 0 {
                println!(
                    "NAVAL study difficulty={difficulty} t={:.0} waves={} rounds={} carrier={:.2}",
                    n.clock, n.waves, max_rounds, n.ships[0].hull
                );
            }
            if n.ended {
                break;
            }
        }
        samples.sort_by(f64::total_cmp);
        let n = c.naval.as_ref().unwrap();
        rows.push(serde_json::json!({"difficulty":difficulty,"simulation_s":n.clock,"waves":n.waves,"defeated":n.defeated,"ship_kills":n.ship_kills,"interceptions":n.interceptions,"carrier_hull":n.ships[0].hull,"carrier_lost":n.ended,"max_rounds":max_rounds,"max_aircraft":max_aircraft,"mean_step_ms":samples.iter().sum::<f64>()/samples.len() as f64,"p95_step_ms":samples[samples.len()*95/100],"player_crashed":f.crashed,"ship_ammunition":n.ships.iter().map(|s|[s.standard as u32,s.essm as u32,s.gun]).collect::<Vec<_>>() }));
    }
    let report = serde_json::json!({"step_hz":120,"requested_seconds":seconds,"note":"Headless current-Mac simulation timings; no rendering or manual pilot input", "runs":rows});
    std::fs::create_dir_all("tests/naval-review").unwrap();
    std::fs::write(
        "tests/naval-review/study.json",
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{report}");
}
/// Disposable native review states; never used by ordinary mission startup.
pub fn prepare_review(mode: &str, f: &mut Flight, c: &mut Combat, v: &mut ViewState) {
    *f = flight(true);
    f.paused = false;
    v.scenario = 15;
    v.generation = 100;
    v.help = false;
    start(c, f, v.generation, 1);
    if mode == "baseline" {
        *c = Combat {
            generation: v.generation,
            ..default()
        };
        v.scenario = 12;
        for _ in 0..600{crate::physics::simulate(f,&c.environment,1./120.);}
    }
    if let Some(mut n) = c.naval.take() {
        // Place the actual raid inside coverage for readable guidance and live launcher review.
        for (i, r) in n.raiders.iter().enumerate() {
            let t = &mut c.targets[r.slot];
            t.position = carrier::world(
                DVec3::new(7000. + i as f64 * 1000., 2500. + i as f64 * 150., -12000.),
                0.,
            );
            t.velocity = carrier::rotation(0.) * DVec3::Z * 200.;
            t.orientation = facing(t.velocity.normalize(), t.position.normalize());
            ai::initialize(t);
            crate::aircraft::select(&mut t.dynamics, crate::aircraft::Aircraft::Su27);
        }
        for _ in 0..600 {
            crate::physics::simulate(f, &c.environment, 1. / 120.);
            c.naval = Some(n);
            c.tick(f, 1. / 120.);
            n = c.naval.take().unwrap();
        }
        if mode == "damage" || mode == "sinking" {
            for j in [0, 2, 4] {
                n.ships[1].compartments[j].fire = 0.65;
                n.ships[1].compartments[j].breach = 0.8;
            }
            if mode == "sinking" {
                n.ships[1].hull = 0.;
                for _ in 0..400 {
                    n.ships[1].advance(0.1);
                }
                n.clock += 0.1;
                n.advance(c, f, 1. / 120.);
            }
        }
        n.report(f);
        println!("NAVAL native review {mode}: {}", n.report);
        c.naval = Some(n);
    }
    if mode == "hud" {
        *f = flight(false);
        f.elapsed = 5.;
        f.paused = true;
        v.index = 3;
        v.yaw = 0.;
        v.zoom = 1.;
        v.pitch = -0.1;
    } else {
        v.ship_follow = Some(if mode == "carrier" {
            0
        } else if mode == "starboard" {
            2
        } else {
            1
        });
        v.index = 1;
        v.yaw = 0.6;
        v.pitch = 0.3;
        v.zoom = 0.65;
    }
    if let Some(n) = &mut c.naval {
        n.report(f);
    }
    f.paused = std::env::var_os("NORD_NAVAL_LIVE").is_none();
}
#[cfg(test)]
mod control_tests {
    use super::*;
    #[test]
    fn radio_keys_are_consumed_and_respawn_preserves_battle() {
        let f = flight(true);
        let mut c = Combat::default();
        start(&mut c, &f, 1, 1);
        c.naval.as_mut().unwrap().ships[1].standard = 3;
        c.naval.as_mut().unwrap().clock = 123.;
        let mut app = App::new();
        app.insert_resource(f)
            .insert_resource(c)
            .insert_resource(ViewState::default())
            .insert_resource(Speech::silent())
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<crate::warp::Warp>()
            .add_systems(
                Update,
                (radio_controls, super::super::camera_controls).chain(),
            );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Backslash);
        app.update();
        assert!(
            app.world()
                .resource::<Combat>()
                .naval
                .as_ref()
                .unwrap()
                .radio_open
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit6);
        app.update();
        assert!(
            !app.world()
                .resource::<ButtonInput<KeyCode>>()
                .just_pressed(KeyCode::Digit6)
        );
        assert!(app.world().resource::<ViewState>().missile_follow.is_none());
        assert!(
            app.world()
                .resource::<Combat>()
                .naval
                .as_ref()
                .unwrap()
                .report
                .contains("RADAR SWEEP")
        );
        app.world_mut().resource_mut::<Flight>().crashed = true;
        app.world_mut().resource_mut::<Flight>().elapsed = 123.;
        app.world_mut()
            .resource_mut::<Combat>()
            .naval
            .as_mut()
            .unwrap()
            .radio_open = true;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit2);
        app.update();
        assert!(!app.world().resource::<Flight>().crashed);
        assert_eq!(app.world().resource::<Flight>().elapsed, 123.);
        let c = app.world().resource::<Combat>();
        let n = c.naval.as_ref().unwrap();
        assert_eq!(n.ships[1].standard, 3);
        assert_eq!(n.clock, 123.);
        assert_eq!(n.raiders.len(), 4);
    }
    #[test]
    fn completed_raids_count_once_and_sinking_continues_after_carrier_loss() {
        let mut f = flight(true);
        let mut c = Combat::default();
        start(&mut c, &f, 1, 1);
        let mut n = c.naval.take().unwrap();
        for t in &mut c.targets {
            t.alive = false;
        }
        n.advance(&mut c, &mut f, 0.01);
        assert_eq!(n.defeated, 1);
        n.advance(&mut c, &mut f, 0.01);
        assert_eq!(n.defeated, 1);
        n.rounds.push(Round{id:99,source:TargetId::Ship(1),target:TargetId::Ship(0),kind:Kind::Standard,position:carrier::world(DVec3::Y*50.,0.),previous:carrier::world(DVec3::Y*50.,0.),velocity:DVec3::Z*100.,age:1.,alive:true,support_lost:0.});
        n.ended=true;n.advance(&mut c,&mut f,0.01);assert!(!n.rounds[0].alive);
        n.ships[0].hull = 0.;
        for _ in 0..200 {
            n.advance(&mut c, &mut f, 0.01);
        }
        assert!(n.ended && n.ships[0].sink > 0.5);
        assert!(n.survival < n.clock);
    }
}
#[cfg(test)]
mod geographic_tests {
    use super::*;
    #[test]
    fn radio_bearing_matches_hud_true_heading_in_every_quadrant() {
        let mut f = flight(false);
        let (n, e) = geographic_axes(f.up());
        for degrees in [0f64, 1., 45., 90., 179., 180., 270., 359.] {
            let d = n * degrees.to_radians().cos() + e * degrees.to_radians().sin();
            f.orientation = facing(d, f.up());
            let (bearing, range) = bearing_range(&f, f.position + d * 18520.);
            let error = (bearing - f.heading() + 180.).rem_euclid(360.) - 180.;
            assert!(
                error.abs() < 1e-8,
                "bearing={bearing} heading={}",
                f.heading()
            );
            assert!((range - 10.).abs() < 1e-7);
        }
    }
}
#[cfg(test)]mod retirement_tests{use super::*;
 #[test]fn destroyed_raid_slots_age_and_reuse_new_stable_id(){let f=flight(true);let mut c=Combat::default();start(&mut c,&f,1,1);let mut n=c.naval.take().unwrap();let old=n.raiders[0].id;c.targets[0].alive=false;n.pilots(&mut c,&f,11.);assert!(c.targets[0].wreck_age>=11.);n.raiders.retain(|r|r.slot!=0);n.clock=500.;n.spawn(&mut c,&f);let new=n.raiders.iter().find(|r|r.slot==0).unwrap();assert_ne!(new.id,old);assert_eq!(c.targets[0].wreck_age,0.);}
}
