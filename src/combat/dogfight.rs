//! Bounded game AI, cannon rounds, countermeasures and combat damage.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Difficulty {
    Training,
    Balanced,
    Hard,
}
impl Difficulty {
    pub fn label(self) -> &'static str {
        match self {
            Self::Training => "TRAINING",
            Self::Balanced => "BALANCED",
            Self::Hard => "HARD",
        }
    }
    fn interval(self) -> f64 {
        match self {
            Self::Training => 18.0,
            Self::Balanced => 11.0,
            Self::Hard => 6.5,
        }
    }
    fn warmup(self) -> f64 {
        match self {
            Self::Training => 14.0,
            Self::Balanced => 6.0,
            Self::Hard => 4.0,
        }
    }
    fn salvo_limit(self) -> usize {
        match self {
            Self::Training => 1,
            Self::Balanced => 2,
            Self::Hard => 4,
        }
    }
    fn damage(self) -> f64 {
        1.0
    }
    fn spread(self) -> f64 {
        match self {
            Self::Training => 0.024,
            Self::Balanced => 0.012,
            Self::Hard => 0.006,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PilotMode {
    Pursue,
    Extend,
    Evade,
    Recover,
}
#[derive(Clone)]
pub(super) struct Pilot {
    countermeasures: [u8; 2],
    decoy_at: f64,
    seeker: Option<Weapon>,
    ammo: [u8; 2],
    gun: u16,
    launch_at: f64,
    dwell: f64,
    gun_at: f64,
    pub mode: PilotMode,
}
impl Pilot {
    pub(super) fn radar_tracking(&self) -> bool {
        self.seeker == Some(Weapon::Radar) && self.dwell > 0.3
    }
}
#[derive(Clone)]
pub struct Decoy {
    pub owner: Option<usize>,
    pub(super) id: u64,
    pub kind: Weapon,
    pub position: DVec3,
    pub(super) velocity: DVec3,
    pub age: f64,
}
#[derive(Clone)]
pub struct Round {
    pub position: DVec3,
    pub previous: DVec3,
    pub(super) velocity: DVec3,
    pub hostile: bool,
    pub age: f64,
    pub(super) damage: f64,
    pub(super) alive: bool,
}
pub(crate) struct Fight {
    pub difficulty: Difficulty,
    pub enemy_missiles: Vec<Missile>,
    pub decoys: Vec<Decoy>,
    pub rounds: Vec<Round>,
    pub flares: u8,
    pub chaff: u8,
    pub gun: u16,
    pub(super) next_decoy: u64,
    flare_at: f64,
    chaff_at: f64,
    pub(super) gun_at: f64,
    pub(super) gun_held: bool,
    last_player: DVec3,
    pub launched: usize,
    pub diverted: usize,
    pub gun_hits: usize,
}

pub fn advance_targets(
    targets: &mut [Target],
    missiles: &[Missile],
    f: &Flight,
    difficulty: Difficulty,
    sun: DVec3,
    player_radar: bool,
    environment: &Environment,
    dt: f64,
) {
    let neighbours: Vec<_> = targets
        .iter()
        .filter(|t| t.alive)
        .map(|t| (t.position, t.velocity))
        .collect();
    for (index, t) in targets.iter_mut().enumerate() {
        if !t.alive {
            t.advance(dt, environment);
            continue;
        }
        ai::sense_player(t, f, difficulty, sun, dt);
        let up = t.position.normalize();
        let dir = (t.orientation * -DVec3::Z).normalize();
        let right = dir.cross(up).normalize_or(DVec3::X);
        let contact = t
            .brain
            .memory
            .map(|(p, v, at)| p + v * (t.brain.elapsed - at));
        let bearing = ai::rf_bearing(t, f, player_radar);
        let goal = contact
            .unwrap_or(t.position + bearing.unwrap_or((dir + right * 0.5).normalize()) * 5000.);
        let r = goal - t.position;
        let range = r.length();
        let estimated_velocity = t.brain.memory.map_or(DVec3::ZERO, |(_, v, _)| v);
        let incoming = missiles
            .iter()
            .filter(|m| ai::warning(t, m, sun))
            .min_by(|a, b| {
                a.position
                    .distance_squared(t.position)
                    .total_cmp(&b.position.distance_squared(t.position))
            });
        let energy = t.velocity.length_squared() * 0.5 + 9.81 * t.dynamics.altitude();
        let opponent_energy = t
            .brain
            .memory
            .map(|(p, v, _)| v.length_squared() * 0.5 + 9.81 * (p.length() - RADIUS));
        let disadvantaged = opponent_energy.is_some_and(|other| energy + 15000. < other);
        let (mode, mut desired, speed, label) = if let Some(m) = incoming {
            let los = (m.position - t.position).normalize();
            (
                PilotMode::Evade,
                (los.cross(up) * if index % 2 == 0 { 1. } else { -1. } + up * 0.1)
                    .normalize_or(dir),
                320.,
                "DEFENSIVE BREAK",
            )
        } else if t.velocity.length() < 180. || disadvantaged && range > 1500. {
            (
                PilotMode::Extend,
                (dir + up * if t.dynamics.agl() > 1500. { -0.12 } else { 0. }).normalize(),
                300.,
                "ENERGY EXTENSION",
            )
        } else if range < 800. && (t.velocity - estimated_velocity).dot(r.normalize_or_zero()) > 60.
        {
            (
                PilotMode::Extend,
                (r.normalize_or(dir) + right * 0.9 + up * 0.2).normalize(),
                220.,
                "LAG / OVERSHOOT",
            )
        } else {
            let lead = if range < 1800. {
                range / CANNON_MUZZLE_SPEED
            } else {
                (range / (t.velocity.length() + estimated_velocity.length()).max(1.)).clamp(0., 4.)
            };
            let predicted = t.brain.memory.map_or(goal, |(_, v, _)| goal + v * lead);
            let vertical = if ai::skill(difficulty) > 0.6
                && range > 1800.
                && t.velocity.length() > 235.
                && opponent_energy.is_some_and(|other| energy > other + 6000.)
            {
                up * range * 0.15
            } else {
                DVec3::ZERO
            };
            let flank = right * range * if index % 2 == 0 { 0.12 } else { -0.12 };
            (
                PilotMode::Pursue,
                (predicted - t.position + vertical + flank).normalize_or(dir),
                270.,
                if vertical.length_squared() > 0. {
                    "VERTICAL / LEAD"
                } else {
                    "LEAD / SUPPORT"
                },
            )
        };
        let mut obstacles = neighbours.clone();
        if t.brain.observed {
            if let Some((p, v, _)) = t.brain.memory {
                obstacles.push((p, v));
            }
        }
        desired =
            (desired + ai::avoidance(t.position, t.velocity, &obstacles) * 1.6).normalize_or(dir);
        t.brain.mode = if contact.is_none() { "SEARCH" } else { label };
        if let Some(pilot) = &mut t.pilot {
            pilot.mode = mode;
        }
        ai::fly(t, environment, desired, speed, difficulty, dt);
    }
}

impl Combat {
    pub fn start_air(&mut self, f: &Flight, generation: u64, scenario: u8) {
        self.start(f, generation);
        let difficulty = match scenario {
            5 => Difficulty::Training,
            6 => Difficulty::Balanced,
            _ => Difficulty::Hard,
        };
        let forward = f.forward();
        let right = f.orientation * DVec3::X;
        let up = f.up();
        for (i, t) in self.targets.iter_mut().enumerate() {
            t.position = f.position
                + forward * (3500.0 + i as f64 * 1600.0)
                + right * ((i as f64 - 1.0) * 850.0)
                + up * (i as f64 * 120.0);
            let direction = (f.position - t.position).normalize();
            t.speed = 245.0;
            t.velocity = direction * t.speed;
            t.orientation = facing(direction, t.position.normalize());
            t.pilot = Some(Pilot {
                countermeasures: [24, 24],
                decoy_at: 0.0,
                seeker: None,
                ammo: [3, 3],
                gun: 300,
                launch_at: difficulty.warmup() + i as f64 * 3.0,
                dwell: 0.0,
                gun_at: 0.0,
                mode: PilotMode::Pursue,
            });
            ai::initialize(t);
        }
        self.fight = Some(Fight {
            difficulty,
            enemy_missiles: vec![],
            decoys: vec![],
            rounds: vec![],
            flares: 24,
            chaff: 24,
            gun: f.aircraft.spec().gun,
            next_decoy: 1,
            flare_at: -10.0,
            chaff_at: -10.0,
            gun_at: 0.0,
            gun_held: false,
            last_player: f.position,
            launched: 0,
            diverted: 0,
            gun_hits: 0,
        });
        self.weapon = Weapon::Radar;
        self.notify(format!(
            "AIR COMBAT / {} / V FLARE + CHAFF · BACKSPACE CHAFF · U GUN",
            difficulty.label()
        ));
    }
    pub(super) fn install_naval_weapons(&mut self,f:&Flight){let difficulty=Difficulty::Balanced;
        self.fight = Some(Fight {
            difficulty,
            enemy_missiles: vec![],
            decoys: vec![],
            rounds: vec![],
            flares: 24,
            chaff: 24,
            gun: f.aircraft.spec().gun,
            next_decoy: 1,
            flare_at: -10.0,
            chaff_at: -10.0,
            gun_at: 0.0,
            gun_held: false,
            last_player: f.position,
            launched: 0,
            diverted: 0,
            gun_hits: 0,
        });
    }
    pub fn difficulty(&self) -> Option<Difficulty> {
        self.fight.as_ref().map(|f| f.difficulty)
    }
    pub fn render_missile(&self, index: usize) -> Option<&Missile> {
        if self.bvr.is_some() {
            return self.missiles.get(index);
        }
        if index < MISSILE_COUNT {
            self.missiles.get(index)
        } else {
            self.fight
                .as_ref()?
                .enemy_missiles
                .get(index - MISSILE_COUNT)
        }
    }
    pub fn missiles_for_draw(&self) -> impl Iterator<Item = &Missile> {
        self.missiles.iter().chain(
            self.fight
                .iter()
                .flat_map(|fight| fight.enemy_missiles.iter()),
        )
    }
    pub fn deploy(&mut self, f: &Flight, kind: Weapon) -> bool {
        if f.paused || f.crashed {
            return false;
        }
        let Some(fight) = &mut self.fight else {
            return false;
        };
        let (ammo, last) = if kind == Weapon::Heat {
            (&mut fight.flares, &mut fight.flare_at)
        } else {
            (&mut fight.chaff, &mut fight.chaff_at)
        };
        if *ammo == 0 || self.clock - *last < 0.6 {
            return false;
        }
        *ammo -= 1;
        *last = self.clock;
        let id = fight.next_decoy;
        fight.next_decoy += 1;
        fight.decoys.push(Decoy {
            owner: None,
            id,
            kind,
            position: f.position + f.orientation * DVec3::new(0.0, -0.3, 3.8),
            velocity: f.velocity - f.forward() * 35.0 - f.up() * 12.0,
            age: 0.0,
        });
        crate::sound::emit(
            &mut self.audio_events,
            crate::sound::Kind::Countermeasure,
            f.position,
            f.velocity,
            true,
        );
        self.notify(if kind == Weapon::Heat {
            "FLARES DISPENSED"
        } else {
            "CHAFF DISPENSED"
        });
        true
    }
    pub fn defense_controls(&mut self, keys: &ButtonInput<KeyCode>, f: &Flight) {
        if let Some(fight) = &mut self.fight {
            fight.gun_held = keys.pressed(KeyCode::KeyU) && !f.paused;
        }
        if keys.just_pressed(KeyCode::KeyV) {
            self.deploy_pair(f);
        }
        if keys.just_pressed(KeyCode::Backspace) {
            self.deploy(f, Weapon::Radar);
        }
    }
    pub fn stop_gun(&mut self) {
        if let Some(fight) = &mut self.fight {
            fight.gun_held = false;
        }
    }
    pub(crate) fn cannon_firing(&self, f: &Flight) -> bool {
        self.cannon_available(f) && self.fight.as_ref().is_some_and(|fight| fight.gun_held)
    }
    pub(crate) fn cannon_available(&self, f: &Flight) -> bool {
        self.active
            && !f.crashed
            && f.battery
            && self
                .fight
                .as_ref()
                .is_some_and(|fight| fight.gun > 0)
    }
    pub fn deploy_pair(&mut self, f: &Flight) -> bool {
        let flare = self.deploy(f, Weapon::Heat);
        let chaff = self.deploy(f, Weapon::Radar);
        self.notify(match (flare, chaff) {
            (true, true) => "FLARES + CHAFF DISPENSED",
            (true, false) => "FLARES / CHAFF EMPTY OR COOLING",
            (false, true) => "CHAFF / FLARES EMPTY OR COOLING",
            _ => "DISPENSERS UNAVAILABLE",
        });
        flare || chaff
    }
    pub fn tick(&mut self, f: &mut Flight, dt: f64) {
        if f.paused {return;}
        self.tick_combat(f,dt);
        if let Some(mut naval)=self.naval.take(){naval.advance(self,f,dt);self.naval=Some(naval);}
        // The rail pose belongs to the completed aircraft step. Release at
        // that endpoint, after existing missiles advance, so a new round is
        // not integrated through an interval preceding its actual release.
        self.service_main_launch(f);
        self.service_side_launch(f);
    }
    fn tick_combat(&mut self, f: &mut Flight, dt: f64) {
        let _measure = crate::performance::span(2);
        if f.paused {
            return;
        }
        for target in &mut self.targets {
            target.dynamics.carrier.active=f.carrier.active;
            target.dynamics.carrier.motion=f.carrier.motion;
            if f.carrier.active {target.dynamics.elapsed=(f.elapsed-dt).max(0.);}
        }
        if self.bvr.is_some() {
            super::bvr::advance(self, f, dt);
            return;
        }
        let mut old = std::mem::take(&mut self.old_poses);
        old.clear();
        old.extend(self.targets.iter().map(|t| (t.position, t.orientation)));
        self.advance(f, dt);
        super::contact::advance(self, f, &old, dt);
        self.old_poses = old;
        if !self.active || f.vehicle != Vehicle::Gripen {
            return;
        }
        let Some(fight) = &mut self.fight else {
            return;
        };
        for decoy in &mut fight.decoys {
            decoy.age += dt;
            let drag = if decoy.kind == Weapon::Heat {
                0.32
            } else {
                1.2
            };
            decoy.velocity *= (-drag * dt).exp();
            decoy.velocity -= decoy.position.normalize() * 9.81 * dt;
            decoy.position += decoy.velocity * dt;
        }
        fight
            .decoys
            .retain(|d| d.age < if d.kind == Weapon::Heat { 5.5 } else { 7.0 });
        let live_missiles = fight.enemy_missiles.iter().filter(|m| m.alive).count();
        let mut permitted = fight.difficulty.salvo_limit().saturating_sub(live_missiles);
        for (i, t) in self.targets.iter_mut().enumerate() {
            if self.naval.is_some(){break;}
            if !t.alive || f.crashed {
                continue;
            }
            let to_player = f.position - t.position;
            let range = to_player.length();
            let aim = to_player.normalize();
            let forward = t.orientation * -DVec3::Z;
            let incoming = self
                .missiles
                .iter()
                .filter(|m| ai::warning(t, m, self.sun))
                .min_by(|a, b| {
                    a.position
                        .distance_squared(t.position)
                        .total_cmp(&b.position.distance_squared(t.position))
                });
            let pilot = t.pilot.as_mut().unwrap();
            if let Some(incoming) = incoming {
                if self.clock >= pilot.decoy_at && pilot.countermeasures[incoming.kind.index()] > 0
                {
                    let kind = incoming.kind;
                    pilot.countermeasures[kind.index()] -= 1;
                    pilot.decoy_at = self.clock
                        + match fight.difficulty {
                            Difficulty::Training => 2.0,
                            Difficulty::Balanced => 1.3,
                            Difficulty::Hard => 0.8,
                        };
                    let id = fight.next_decoy;
                    fight.next_decoy += 1;
                    fight.decoys.push(Decoy {
                        owner: Some(i),
                        id,
                        kind,
                        position: t.position - forward * 3.8,
                        velocity: t.velocity - forward * 35.0 - t.position.normalize() * 12.0,
                        age: 0.0,
                    });
                }
            }
            let kind = if range < 5000.0 && pilot.ammo[0] > 0 {
                Weapon::Heat
            } else {
                Weapon::Radar
            };
            let can_lock = t.brain.observed
                && range > 350.0
                && range < kind.range()
                && forward.dot(aim) > 24.0_f64.to_radians().cos()
                && pilot.mode != PilotMode::Evade
                && pilot.mode != PilotMode::Recover
                && pilot.ammo[kind.index()] > 0
                && (kind != Weapon::Radar || t.damage.radar > 0.25)
                && super::guidance::quality(kind, t.position, super::guidance::player(f), self.sun)
                    .0
                    >= 0.18;
            pilot.dwell = if can_lock {
                pilot.dwell
                    + dt * super::guidance::quality(
                        kind,
                        t.position,
                        super::guidance::player(f),
                        self.sun,
                    )
                    .0
            } else {
                0.0
            };
            pilot.seeker = if can_lock { Some(kind) } else { None };
            if can_lock
                && self.clock >= pilot.launch_at
                && pilot.dwell > kind.lock_time()
                && permitted > 0
            {
                let position = t.position
                    + t.orientation * DVec3::new(if i % 2 == 0 { -2.5 } else { 2.5 }, -0.5, 0.7);
                fight.enemy_missiles.push(Missile {
                    gnc: super::guidance::Gnc::launch(
                        t.velocity + forward * 18.0,
                        super::guidance::player(f),
                        self.generation.wrapping_mul(997) + fight.launched as u64 + 101,
                    ).inherit(t.orientation,t.orientation*t.dynamics.rates),
                    position,
                    velocity: t.velocity + (t.orientation*t.dynamics.rates).cross(position-t.position) + forward * 18.0 - t.position.normalize() * 4.0,
                    kind,
                    target: i,
                    age: 0.0,
                    alive: true,
                    guided: true,
                    lost_for: 0.0,
                    trail: vec![position],
                    trail_clock: 0.0,
                    decoy: None,
                });
                crate::sound::emit(
                    &mut self.audio_events,
                    crate::sound::Kind::Launch,
                    position,
                    t.velocity,
                    false,
                );
                pilot.ammo[kind.index()] -= 1;
                pilot.launch_at = self.clock + fight.difficulty.interval();
                pilot.dwell = 0.0;
                fight.launched += 1;
                permitted -= 1;
            }
            let lead =
                (f.position + f.velocity * (range / CANNON_MUZZLE_SPEED) - t.position).normalize();
            if t.brain.observed
                && range < 1600.0
                && self.clock > fight.difficulty.warmup()
                && self.clock >= pilot.gun_at
                && pilot.gun > 0
                && forward.dot(lead) > 1.2_f64.to_radians().cos()
                && pilot.mode != PilotMode::Evade
            {
                let shot_direction = spread(
                    forward,
                    t.position.normalize(),
                    pilot.gun as f64 + i as f64 * 13.0,
                    fight.difficulty.spread(),
                );
                let position = t.position + t.orientation * DVec3::new(0.35, -0.1, -6.0);
                fight.rounds.push(Round {
                    position,
                    previous: position,
                    velocity: t.velocity + shot_direction * CANNON_MUZZLE_SPEED,
                    hostile: true,
                    age: 0.0,
                    damage: 0.085 * fight.difficulty.damage(),
                    alive: true,
                });
                crate::sound::emit(
                    &mut self.audio_events,
                    crate::sound::Kind::Cannon,
                    position,
                    t.velocity,
                    false,
                );
                pilot.gun -= 1;
                pilot.gun_at = self.clock + 0.10;
            }
        }
        if fight.gun_held && !f.crashed && f.battery && fight.gun > 0 && self.clock >= fight.gun_at
        {
            let direction = spread(f.forward(), f.up(), fight.gun as f64, 0.0025);
            let position = f.position + f.orientation * crate::aircraft::gun_origin(f.aircraft);
            fight.rounds.push(Round {
                position,
                previous: position,
                velocity: f.velocity + direction * CANNON_MUZZLE_SPEED,
                hostile: false,
                age: 0.0,
                damage: 0.075,
                alive: true,
            });
            crate::sound::emit(
                &mut self.audio_events,
                crate::sound::Kind::Cannon,
                position,
                f.velocity,
                true,
            );
            fight.gun -= 1;
            fight.gun_at = self.clock + f.aircraft.spec().gun_interval;
        }
        for m in &mut fight.enemy_missiles {
            if !m.alive {
                continue;
            }
            let old = m.position;
            let aircraft = super::guidance::player(f);
            if m.gnc.gimbal.length_squared() < 0.5 {
                m.gnc.gimbal = m.velocity.normalize_or_zero();
            }
            if m.decoy.is_none() && (m.kind == Weapon::Heat || m.gnc.active) {
                let kind = m.kind;
                for d in fight
                    .decoys
                    .iter()
                    .filter(|d| d.kind == kind && d.owner.is_none())
                {
                    let contact = decoy_contact(d);
                    if super::guidance::consider_decoy(m, aircraft, contact, d.age, d.id, self.sun)
                    {
                        m.decoy = Some(d.id);
                        fight.diverted += 1;
                        break;
                    }
                }
            }
            if m.decoy.is_some_and(|id| {
                !fight.decoys.iter().any(|d| {
                    d.id == id
                        && super::guidance::retain_decoy(
                            m,
                            aircraft,
                            decoy_contact(d),
                            d.age,
                            self.sun,
                        )
                })
            }) {
                m.decoy = None;
            }
            let contact = m
                .decoy
                .and_then(|id| fight.decoys.iter().find(|d| d.id == id))
                .map_or(aircraft, decoy_contact);
            let launcher = &self.targets[m.target];
            let support = super::guidance::Support {
                position: launcher.position,
                forward: launcher.orientation * -DVec3::Z,
                available: launcher.alive
                    && launcher.damage.radar > 0.25
                    && launcher.damage.avionics > 0.25,
            };
            m.gnc.wind = self.environment.wind(m.position);
            super::guidance::fly(m, contact, support, self.sun, dt);
            if let Some(hit)=crate::carrier::collider::sweep(f,old,m.position,dt,0.12) {
                m.position=hit.point;m.alive=false;
                self.impacts.push(Impact{position:hit.point,age:0.,hit:false});
                continue;
            }
            let (closest, _) =
                super::damage::closest_relative(old - fight.last_player, m.position - f.position);
            let distance = closest.length();
            let body = f.orientation.conjugate() * closest;
            if m.age > 0.4
                && !f.crashed
                && distance < m.kind.fuse() + 9.0
                && crate::airframe::distance_to_aircraft(body,f.aircraft) < m.kind.fuse()
            {
                m.alive = false;
                let body = f.orientation.conjugate() * closest;
                let angles = crate::aircraft::angles(f);
                let event =
                    super::damage::missile(&mut f.damage, &mut f.integrity, body, m.kind, angles);
                finalize_player_damage(f);
                if !f.crashed {
                    f.status = event.label();
                }
                self.impacts.push(Impact {
                    position: f.position + closest,
                    age: 0.0,
                    hit: true,
                });
            } else if m.position.length() - RADIUS
                < terrain_height(m.position.x, m.position.z).max(0.0)
                || m.age > 45.0
            {
                m.alive = false;
                self.impacts.push(Impact {
                    position: m.position,
                    age: 0.0,
                    hit: false,
                });
            }
        }
        for round in &mut fight.rounds {
            let old = round.position;
            round.previous = old;
            round.age += dt;
            round.velocity -= round.position.normalize() * 9.81 * dt;
            round.position += round.velocity * dt;
        if let Some(hit)=crate::carrier::collider::sweep(f,old,round.position,dt,0.02) {
            round.position=hit.point;round.alive=false;continue;
        }
            if round.hostile && !f.crashed {
                let start = f.orientation.conjugate() * (old - fight.last_player);
                let end = f.orientation.conjugate() * (round.position - f.position);
                if let Some(body) = bullet_hit(f, start, end) {
                    round.alive = false;
                    fight.gun_hits += 1;
                    let relative_speed = (round.velocity - f.velocity).length();
                    apply_hit(
                        f,
                        body,
                        round.damage
                            * (relative_speed / CANNON_MUZZLE_SPEED)
                                .powi(2)
                                .clamp(0.25, 2.0),
                        false,
                    );
                }
            } else if !round.hostile {
                for (index, target) in self.targets.iter_mut().enumerate() {
                    if target.alive
                        && let Some(body) = target_bullet_hit(target, old, round.position, dt)
                    {
                        round.alive = false;
                        self.strikes += 1;
                        let relative_speed = (round.velocity - target.velocity).length();
                        super::damage::enemy_projectile(
                            &mut target.damage,
                            &mut target.health,
                            body,
                            round.damage
                                * (relative_speed / CANNON_MUZZLE_SPEED)
                                    .powi(2)
                                    .clamp(0.25, 2.0),
                        );
                        if super::damage::fatal(&target.damage, target.health) {
                            target.alive = false;
                            target.wreck_age = 0.0;
                            self.hits += 1;
                            if let Some(n)=&mut self.naval{n.player_kills+=1;}
                            self.impacts.push(Impact {
                                position: target.position,
                                age: 0.0,
                                hit: true,
                            });
                        }
                        self.recent_hit =
                            Some(super::effects::HitReport::new(index, target, self.clock));
                        break;
                    }
                }
            }
        }
        fight.rounds.retain(|r| {
            r.alive
                && r.age < 2.5
                && r.position.length() - RADIUS
                    > terrain_height(r.position.x, r.position.z).max(0.0)
        });
        if fight.rounds.len() > 128 {
            fight.rounds.drain(0..fight.rounds.len() - 128);
        }
        fight.last_player = f.position;
        if self.targets.get(self.selected).is_some_and(|t| !t.alive) {
            self.cycle();
        }
    }
    pub fn threats(&self, f: &Flight) -> (usize, f64, f64, Option<Weapon>) {
        if let Some(b) = &self.bvr {
            let mut live = self.missiles.iter().zip(&b.shots).filter(|(m, s)| {
                m.alive
                    && !s.friendly
                    && s.target.is_none()
                    && (m.gnc.active || m.position.distance(f.position) < 35000.)
            });
            if let Some((nearest, _)) = live.clone().min_by(|a, z| {
                a.0.position
                    .distance_squared(f.position)
                    .total_cmp(&z.0.position.distance_squared(f.position))
            }) {
                let r = f.orientation.conjugate() * (nearest.position - f.position);
                return (
                    live.by_ref().count(),
                    r.length(),
                    r.x.atan2(-r.z).to_degrees(),
                    Some(nearest.kind),
                );
            }
            return (0, 0., 0., None);
        }
        let Some(fight) = &self.fight else {
            return (0, 0.0, 0.0, None);
        };
        let live: Vec<_> = fight
            .enemy_missiles
            .iter()
            .filter(|m| {
                m.alive
                    && (m.decoy.is_none() || m.kind == Weapon::Heat)
                    && (m.velocity - f.velocity).dot(f.position - m.position) > 0.0
            })
            .collect();
        let nearest = live.iter().min_by(|a, b| {
            a.position
                .distance_squared(f.position)
                .total_cmp(&b.position.distance_squared(f.position))
        });
        if let Some(m) = nearest {
            let r = f.orientation.conjugate() * (m.position - f.position);
            (
                live.len(),
                r.length(),
                r.x.atan2(-r.z).to_degrees(),
                Some(m.kind),
            )
        } else {
            (0, 0.0, 0.0, None)
        }
    }
}

pub(super) fn bullet_hit(f: &Flight, start: DVec3, end: DVec3) -> Option<DVec3> {
    if swept_distance(start, end) > f.aircraft.spec().length*0.65 {
        return None;
    }
    let half = DVec3::splat(0.045);
    crate::airframe::for_flight(f)
        .iter()
        .filter(|h| h.enabled(f))
        .filter_map(|hull| {
            let (rotation, offset) = hull.pose(f);
            hull.sweep_box(
                offset,
                start - end,
                rotation,
                DVec3::ZERO,
                start - half,
                start + half,
                0.0,
            )
            .map(|(time, _)| time)
        })
        .min_by(f64::total_cmp)
        .map(|time| start + (end - start) * time)
}
pub(super) fn target_bullet_hit(t: &Target, old: DVec3, new: DVec3, dt: f64) -> Option<DVec3> {
    let start = t.orientation.conjugate() * (old - (t.position - t.velocity * dt));
    let end = t.orientation.conjugate() * (new - t.position);
    if swept_distance(start, end) > 8.0 {
        return None;
    }
    let shapes = [
        (DVec3::ZERO, DVec3::new(0.85, 0.72, 6.5)),
        (DVec3::new(-2.6, 0.0, 1.1), DVec3::new(3.6, 0.18, 1.8)),
        (DVec3::new(2.6, 0.0, 1.1), DVec3::new(3.6, 0.18, 1.8)),
        (DVec3::new(0.0, 0.9, 4.5), DVec3::new(0.45, 2.0, 0.9)),
        (DVec3::new(-1.4, 0.0, -2.7), DVec3::new(1.8, 0.15, 0.8)),
        (DVec3::new(1.4, 0.0, -2.7), DVec3::new(1.8, 0.15, 0.8)),
    ];
    shapes
        .into_iter()
        .filter_map(|(center, radius)| {
            let a = (start - center) / radius;
            let d = (end - start) / radius;
            let aa = d.length_squared();
            if aa < 1e-12 {
                return (a.length_squared() < 1.0).then_some(0.0);
            }
            let b = 2.0 * a.dot(d);
            let c = a.length_squared() - 1.0;
            if c <= 0.0 {
                return Some(0.0);
            }
            let disc = b * b - 4.0 * aa * c;
            (disc >= 0.0)
                .then(|| (-b - disc.max(0.0).sqrt()) / (2.0 * aa))
                .filter(|time| (0.0..=1.0).contains(time))
        })
        .min_by(f64::total_cmp)
        .map(|time| start + (end - start) * time)
}
fn level_controls(f: &mut Flight, altitude: f64) {
    f.controls = DVec3::new(
        ((1.0 - f.g_load) * 0.14 - f.vertical_speed() * 0.005
            + (altitude - f.altitude()) * 0.00008)
            .clamp(-0.4, 0.4),
        -f.beta * 2.0,
        (-f.roll() * 2.0).clamp(-0.5, 0.5),
    );
    f.throttle = (0.48 + (230.0 - f.airspeed) * 0.002).clamp(0.15, 0.88);
}
impl Combat {
    pub fn prepare_air_capture(&mut self, f: &mut Flight, mode: &str) {
        let mut e = Environment::default();
        e.wind_enabled = false;
        let steps = if matches!(
            mode,
            "air-missile" | "air-defenses" | "air-damage" | "air-follow" | "air-hud-threat"
        ) {
            1800
        } else {
            180
        };
        for _ in 0..steps {
            level_controls(f, 2000.0);
            e.advance(1.0 / 120.0);
            simulate(f, &e, 1.0 / 120.0);
            self.tick(f, 1.0 / 120.0);
            if self.fight.as_ref().is_some_and(|fight| fight.launched > 0) {
                break;
            }
        }
        if mode == "air-defenses" {
            self.deploy(f, Weapon::Heat);
            self.deploy(f, Weapon::Radar);
            for _ in 0..72 {
                level_controls(f, 2000.0);
                e.advance(1.0 / 120.0);
                simulate(f, &e, 1.0 / 120.0);
                self.tick(f, 1.0 / 120.0);
            }
        }
        if mode == "air-damage" {
            apply_hit(f, DVec3::new(0.0, 0.0, 4.0), 0.45, false);
            apply_hit(f, DVec3::new(-3.0, 0.0, 2.7), 0.20, false);
            apply_hit(f, DVec3::new(0.0, 0.0, -3.0), 0.50, false);
            simulate(f, &e, 1.0 / 120.0);
        }
        if mode == "air-target-damage" {
            if let Some(target) = self.targets.first_mut() {
                super::damage::blast(
                    &mut target.damage,
                    &mut target.health,
                    DVec3::new(-6.0, 1.0, 2.0),
                    Weapon::Heat,
                );
            }
        }
        if matches!(
            mode,
            "air-hud" | "air-hud-cockpit" | "air-hud-simple" | "air-hud-cockpit-simple"
        ) {
            let target = &mut self.targets[0];
            target.position =
                f.position + f.forward() * 1100.0 + f.orientation * DVec3::new(45.0, 12.0, 0.0);
            target.velocity = f.velocity + f.orientation * DVec3::X * 35.0;
            target.orientation = facing(target.velocity.normalize(), target.position.normalize());
            self.lock = self.weapon.lock_time() * 0.72;
            f.rates.x = 0.18;
        }
        if mode.starts_with("air-component-") {
            match mode {
                "air-component-detail" => {
                    f.damage.engine = 0.2;
                    f.damage.wings[0] = 0.45;
                    f.damage.structure[0] = 0.55;
                    f.damage.radar = 0.4;
                    f.damage.fuel_leak = 1.8;
                    f.damage.fire = 0.7;
                    f.engine = false;
                    f.damage.surface_state[2] = super::damage::SurfaceState::Jammed(0.25);
                    f.damage.surface_state[4] = super::damage::SurfaceState::Detached;
                }
                "air-component-jam" => {
                    f.damage.surface_state[2] = super::damage::SurfaceState::Jammed(0.35);
                    f.damage.surfaces[2] = 0.8;
                }
                "air-component-detached" => {
                    f.damage.surface_state[2] = super::damage::SurfaceState::Detached;
                    f.damage.surfaces[2] = 0.0;
                    f.damage.wings[0] = 0.7;
                }
                "air-component-fire" | "air-component-cockpit-simple" => {
                    f.damage.engine = 0.0;
                    f.damage.fire = 0.8;
                    f.damage.fuel_leak = 1.4;
                    f.engine = false;
                    f.damage.surface_state[2] = super::damage::SurfaceState::Jammed(0.25);
                    f.damage.surface_state[4] = super::damage::SurfaceState::Detached;
                }
                "air-component-breakup" => {
                    crate::collision::destroy(f, "MISSILE / CATASTROPHIC HIT", f.up())
                }
                "air-component-collision" => {
                    let a = self.targets[0].position;
                    self.targets[1].position = a + f.orientation * DVec3::X * 0.5;
                    self.targets[1].velocity =
                        self.targets[0].velocity + f.orientation * DVec3::X * 60.0;
                    let old = self
                        .targets
                        .iter()
                        .map(|t| (t.position, t.orientation))
                        .collect::<Vec<_>>();
                    super::contact::advance(self, f, &old, 1.0 / 120.0);
                }
                _ => {}
            }
            for _ in 0..if mode == "air-component-detail" {
                180
            } else {
                30
            } {
                simulate(f, &e, 1.0 / 120.0);
                self.tick(f, 1.0 / 120.0);
                super::effects::advance(self, f, 1.0 / 120.0);
            }
        }
        if mode == "air-follow" {
            // Inspect an actual missile coast after it clears the launching aircraft.
            for _ in 0..90 {
                level_controls(f, 2000.0);
                e.advance(1.0 / 120.0);
                simulate(f, &e, 1.0 / 120.0);
                self.tick(f, 1.0 / 120.0);
            }
        }
    }
}
pub fn study() {
    let mut runs = Vec::new();
    for scenario in 5..=7 {
        for defend in [false, true] {
            let mut f = flight();
            let mut e = Environment::default();
            e.wind_enabled = false;
            let mut c = Combat::default();
            c.start_air(&f, 1, scenario);
            let mut next_defense = 0.0;
            for _ in 0..14400 {
                level_controls(&mut f, 2000.0);
                if defend && c.clock >= next_defense {
                    let (_, range, _, kind) = c.threats(&f);
                    if let Some(kind) = kind.filter(|_| range < 2000.0) {
                        c.deploy(&f, kind);
                        next_defense = c.clock + 0.75;
                    }
                }
                e.advance(1.0 / 120.0);
                simulate(&mut f, &e, 1.0 / 120.0);
                c.tick(&mut f, 1.0 / 120.0);
                if f.crashed {
                    break;
                }
            }
            let fight = c.fight.as_ref().unwrap();
            let guns_fired = 900
                - c.targets
                    .iter()
                    .filter_map(|t| t.pilot.as_ref())
                    .map(|p| p.gun as usize)
                    .sum::<usize>();
            assert!(
                f.position.is_finite()
                    && f.velocity.is_finite()
                    && c.targets
                        .iter()
                        .all(|t| t.position.is_finite() && t.velocity.is_finite())
            );
            runs.push(serde_json::json!({"difficulty":fight.difficulty.label(),"automatic_defense":defend,
            "simulated_seconds":f.elapsed,"player_destroyed":f.crashed,"hull":f.integrity,"engine":f.damage.engine,
            "radar":f.damage.radar,"avionics":f.damage.avionics,"hydraulics":f.damage.hydraulics,
            "surface_health":f.damage.surfaces,"enemy_heat_launches":fight.enemy_missiles.iter().filter(|m|m.kind==Weapon::Heat).count(),
            "enemy_radar_launches":fight.enemy_missiles.iter().filter(|m|m.kind==Weapon::Radar).count(),
            "enemy_cannon_rounds":guns_fired,"cannon_hits":fight.gun_hits,"missiles_diverted":fight.diverted,
            "flares_remaining":fight.flares,"chaff_remaining":fight.chaff}));
        }
    }
    let report = serde_json::json!({"description":"Deterministic game combat checks with a level/speed feedback controller; no offensive player fire", "runs":runs});
    std::fs::write(
        "tests/air-combat-study.json",
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
fn spread(direction: DVec3, up: DVec3, seed: f64, amount: f64) -> DVec3 {
    let right = direction.cross(up).normalize_or_zero();
    (direction + right * (seed * 2.399).sin() * amount + up * (seed * 1.713).cos() * amount)
        .normalize()
}

pub(super) fn decoy_contact(d: &Decoy) -> super::guidance::Contact {
    let heat = d.kind == Weapon::Heat;
    super::guidance::Contact {
        position: d.position,
        velocity: d.velocity,
        forward: d.velocity.normalize_or_zero(),
        throttle: 1.0,
        engine: heat,
        alive: d.age < 4.0,
        decoy: true,
        strength: if heat {
            12.0 * (-d.age / 1.1).exp()
        } else {
            2.5 * (-d.age / 2.5).exp()
        },
    }
}
pub fn apply_hit(f: &mut Flight, body: DVec3, severity: f64, missile: bool) {
    if f.crashed {
        return;
    }
    if missile {
        super::damage::blast_scaled(&mut f.damage, &mut f.integrity, body, severity);
    } else {
        super::damage::projectile(&mut f.damage, &mut f.integrity, body, severity);
    }
    finalize_player_damage(f);
}
pub(super) fn finalize_player_damage(f: &mut Flight) {
    if f.damage.engine < 0.12 {
        f.engine = false;
        f.thrust = 0.0;
    }
    if f.damage.avionics < 0.25 {
        f.assist = false;
        f.autopilot = false;
    }
    if super::damage::fatal(&f.damage, f.integrity) {
        crate::collision::destroy(f, "COMBAT DAMAGE / AIRFRAME LOST", f.up());
    } else {
        f.status = format!(
            "COMBAT DAMAGE / HULL {:.0}% · ENGINE {:.0}% · LEFT/RIGHT WING {:.0}/{:.0}%",
            f.integrity * 100.0,
            f.damage.engine * 100.0,
            f.damage.wings[0] * 100.0,
            f.damage.wings[1] * 100.0
        );
    }
}

#[derive(Component)]
pub struct DefensePanel;
#[derive(Component)]
pub struct DefenseText;
#[derive(Component)]
pub struct DefenseButton(u8);
#[derive(Component)]
pub struct DecoyCloud;

pub fn setup(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let panel = c
        .spawn((
            DefensePanel,
            Node {
                position_type: PositionType::Absolute,
                right: px(28),
                bottom: px(174),
                width: px(284),
                padding: UiRect::all(px(10)),
                border: UiRect::left(px(2)),
                border_radius: BorderRadius::all(px(7)),
                flex_direction: FlexDirection::Column,
                row_gap: px(7),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.055, 0.072, 0.92)),
            BorderColor::all(Color::srgb(0.93, 0.53, 0.30)),
            ZIndex(31),
        ))
        .id();
    let text = c
        .spawn((
            DefenseText,
            Text::new(""),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(Color::srgb(0.80, 0.92, 0.90)),
        ))
        .id();
    c.entity(panel).add_child(text);
    let row = c
        .spawn(Node {
            column_gap: px(5),
            ..default()
        })
        .id();
    c.entity(panel).add_child(row);
    for (tag, label) in [(0, "FLARES ONLY"), (1, "BKSP CHAFF")] {
        let button = c
            .spawn((
                Button,
                DefenseButton(tag),
                Node {
                    width: px(129),
                    height: px(26),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.07, 0.20, 0.24)),
            ))
            .id();
        let text = c
            .spawn((
                Text::new(label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.87, 0.95, 0.95)),
            ))
            .id();
        c.entity(button).add_child(text);
        c.entity(row).add_child(button);
    }
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::new(3.0, 1.2, 0.2, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    c.spawn((
        DecoyCloud,
        Mesh3d(meshes.add(super::mesh(
            vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            vec![0, 1, 2],
        ))),
        MeshMaterial3d(material),
        Transform::default(),
        Visibility::Hidden,
        bevy::camera::visibility::NoFrustumCulling,
        bevy::light::NotShadowCaster,
    ));
}
pub fn buttons(
    mut buttons: Query<(&Interaction, &DefenseButton, &mut BackgroundColor), Changed<Interaction>>,
    f: Res<Flight>,
    v: Res<ViewState>,
    warp: Res<crate::warp::Warp>,
    mut combat: ResMut<Combat>,
    preview: Option<Res<crate::VisualCheck>>,
) {
    if warp.editing || preview.is_some_and(|p| p.0.is_some()) {
        return;
    }
    for (interaction, tag, mut color) in &mut buttons {
        color.0 = if *interaction == Interaction::None {
            Color::srgb(0.07, 0.20, 0.24)
        } else {
            Color::srgb(0.15, 0.34, 0.30)
        };
        if *interaction == Interaction::Pressed && !v.left_camera_drag {
            combat.deploy(
                &f,
                if tag.0 == 0 {
                    Weapon::Heat
                } else {
                    Weapon::Radar
                },
            );
        }
    }
}
#[derive(Default)]
pub struct DecoyBuffers { positions: Vec<Vec3>, colors: Vec<[f32;4]> }
fn write_decoys(mesh: &mut Mesh, positions: &[Vec3], colors: &[[f32;4]]) {
    use bevy::mesh::VertexAttributeValues;
    let count=positions.len();
    if !mesh.contains_attribute(Mesh::ATTRIBUTE_COLOR) { mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR,Vec::<[f32;4]>::new()); }
    for (attribute,values) in mesh.attributes_mut() {
        match values {
            VertexAttributeValues::Float32x3(v) if attribute.id==Mesh::ATTRIBUTE_POSITION.id => {v.clear();v.extend(positions.iter().map(|p|p.to_array()));},
            VertexAttributeValues::Float32x4(v) if attribute.id==Mesh::ATTRIBUTE_COLOR.id => {v.clear();v.extend_from_slice(colors);},
            VertexAttributeValues::Float32x3(v) if attribute.id==Mesh::ATTRIBUTE_NORMAL.id => v.resize(count,[0.,0.,1.]),
            VertexAttributeValues::Float32x2(v) if attribute.id==Mesh::ATTRIBUTE_UV_0.id => v.resize(count,[0.,0.]),
            _=>{}
        }
    }
    let triangles=count/5*12;
    if let Some(Indices::U32(indices))=mesh.indices_mut() {
        if indices.len()!=triangles {
            indices.clear();
            for i in 0..count/5 { let n=i as u32*5;indices.extend([n,n+1,n+2,n,n+2,n+3,n,n+3,n+4,n,n+4,n+1]); }
        }
    }
    // Unlit, two-sided, NotShadowCaster billboards do not consume normals.
    // Preserve all positions, vertex colors, UVs and triangle order.
}
pub fn draw(
    f: Res<Flight>,
    v: Res<ViewState>,
    combat: Res<Combat>,
    mut panels: Query<&mut Node, With<DefensePanel>>,
    mut readouts: Query<(&mut Text, &mut TextColor), With<DefenseText>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut clouds: Query<(&Mesh3d, &mut Visibility), With<DecoyCloud>>,
    camera: Query<&GlobalTransform, With<FlightCamera>>,
    mut gizmos: Gizmos,
    mut buffers: Local<DecoyBuffers>,
) {
    let active = combat.fight.is_some() && combat.active && !v.map && f.vehicle == Vehicle::Gripen;
    for mut panel in &mut panels {
        panel.display = if active && !v.simple && v.index != 3 && v.ship_follow.is_none() {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (_, mut visibility) in &mut clouds {
        *visibility = if active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if !active {
        return;
    }
    let fight = combat.fight.as_ref().unwrap();
    let (count, range, bearing, kind) = combat.threats(&f);
    let radar_locks = combat.bvr.as_ref().map_or_else(
        || {
            combat
                .targets
                .iter()
                .filter(|t| {
                    t.alive
                        && t.pilot
                            .as_ref()
                            .is_some_and(|p| p.dwell > 1.3 && p.seeker == Some(Weapon::Radar))
                })
                .count()
        },
        |b| b.tracking_player(&combat),
    );
    let warning = if f.crashed {
        "AIRFRAME LOST / R RESTART".into()
    } else if let Some(kind) = kind {
        format!(
            "{} INBOUND {:.1} km · {:+.0}°\n{}",
            if kind == Weapon::Heat { "IR" } else { "RADAR" },
            range / 1000.0,
            bearing,
            if kind == Weapon::Heat {
                "V FLARE + CHAFF / BREAK TURN"
            } else {
                "BKSP CHAFF / BREAK TURN"
            }
        )
    } else if radar_locks > 0 {
        format!("RADAR LOCK / {} TRACKING YOU", radar_locks)
    } else {
        "NO INCOMING MISSILE".into()
    };
    for (mut text, mut color) in &mut readouts {
        let readout = format!(
            "DEFENSE / {} · THREATS {}\n{}\nFLARES {} · CHAFF {} · GUN {}\nENG {:.0}% · HYD {:.0}% · RAD {:.0}%\nAVIONICS {:.0}% · CTRL {:.0}% · HULL {:.0}%\nWINGS {:.0}/{:.0}% · FIRE {:.0}%",
            fight.difficulty.label(),
            count,
            warning,
            fight.flares,
            fight.chaff,
            fight.gun,
            f.damage.engine * 100.0,
            f.damage.hydraulics * 100.0,
            f.damage.radar * 100.0,
            f.damage.avionics * 100.0,
            f.damage.surfaces.iter().copied().fold(1.0, f64::min) * 100.0,
            f.integrity * 100.0,
            f.damage.wings[0] * 100.0,
            f.damage.wings[1] * 100.0,
            f.damage.fire * 100.0
        );
        // Assigning unchanged text still marks it changed and forces a re-layout.
        if text.0 != readout {
            text.0 = readout;
        }
        color.set_if_neq(TextColor(if count > 0 && (combat.clock * 3.0).fract() > 0.5 {
            Color::srgb(1.0, 0.40, 0.20)
        } else {
            Color::srgb(0.80, 0.92, 0.90)
        }));
    }
    for round in &fight.rounds {
        let tip = world::local(round.position, &f);
        let tail = world::local(round.position - round.velocity.normalize() * 12.0, &f);
        gizmos.line(
            tail,
            tip,
            if round.hostile {
                Color::srgb(1.0, 0.34, 0.12)
            } else {
                Color::srgb(1.0, 0.94, 0.50)
            },
        );
    }
    let camera = camera.iter().next();
    let right = camera.map_or(Vec3::X, |c| c.rotation() * Vec3::X);
    let up = camera.map_or(Vec3::Y, |c| c.rotation() * Vec3::Y);
    let DecoyBuffers {positions, colors}=&mut *buffers;
    positions.clear(); colors.clear();
    for decoy in &fight.decoys {
        let heat = decoy.kind == Weapon::Heat;
        let fade = (1.0 - decoy.age / if heat { 5.5 } else { 7.0 }).max(0.0) as f32;
        for i in 0..if heat { 6 } else { 12 } {
            let angle = i as f64 * 2.39996;
            let offset = DVec3::new(angle.cos(), angle.sin() * 0.5, angle.sin())
                * (decoy.age * if heat { 18.0 } else { 8.0 });
            let center = world::local(decoy.position + offset, &f);
            let size = if heat {
                0.65 + decoy.age as f32 * 0.12
            } else {
                0.4 + decoy.age as f32 * 0.15
            };
            positions.push(center);
            colors.push(if heat {
                [1.0, 0.85, 0.32, fade]
            } else {
                [0.72, 0.85, 0.94, fade * 0.55]
            });
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                positions.push(center + (right * x + up * y) * size);
                colors.push([1.0, 0.6, 0.1, 0.0]);
            }
            if heat && decoy.age > 0.1 {
                gizmos.line(
                    center,
                    world::local(
                        decoy.position + offset - decoy.velocity.normalize() * 4.0,
                        &f,
                    ),
                    Color::srgba(1.0, 0.6, 0.20, fade * 0.5),
                );
            }
        }
    }
    if positions.is_empty() {
        for (_, mut visibility) in &mut clouds {
            *visibility = Visibility::Hidden;
        }
    } else {
        for (handle, _) in &mut clouds {
            if let Some(mesh) = meshes.get_mut(&handle.0) {
                write_decoys(mesh,positions,colors);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reusable_decoy_mesh_preserves_billboard_topology_and_pixels_inputs() {
        let mut mesh=super::super::mesh(vec![Vec3::ZERO,Vec3::X,Vec3::Y],vec![0,1,2]);
        for count in [5,180,10,0,250,250] {
            let positions:Vec<_>=(0..count).map(|i|Vec3::new(i as f32*0.12,(i%3) as f32,0.)).collect();
            let colors=vec![[1.,0.6,0.1,0.3];count];
            write_decoys(&mut mesh,&positions,&colors);
            let mut expected_indices=Vec::new();
            for i in 0..count/5 {let n=i as u32*5;expected_indices.extend([n,n+1,n+2,n,n+2,n+3,n,n+3,n+4,n,n+4,n+1]);}
            let reference=super::super::mesh(positions,expected_indices).with_inserted_attribute(Mesh::ATTRIBUTE_COLOR,colors);
            for attr in [Mesh::ATTRIBUTE_POSITION,Mesh::ATTRIBUTE_COLOR,Mesh::ATTRIBUTE_UV_0] {
                assert_eq!(mesh.attribute(attr),reference.attribute(attr));
            }
            assert_eq!(mesh.indices(),reference.indices());
        }
    }
    #[test]
    fn cannon_audio_sustain_requires_available_firing_system() {
        let mut f = flight();
        let mut c = Combat::default();
        c.start_air(&f, 1, 5);
        c.fight.as_mut().unwrap().gun_held = true;
        assert!(c.cannon_firing(&f));
        f.battery = false;
        assert!(!c.cannon_firing(&f));
        f.battery = true;
        f.crashed = true;
        assert!(!c.cannon_firing(&f));
        f.crashed = false;
        c.fight.as_mut().unwrap().gun = 0;
        assert!(!c.cannon_firing(&f));
        c.fight.as_mut().unwrap().gun = 1;
        c.stop_gun();
        assert!(!c.cannon_firing(&f));
    }
    #[test]
    fn cannon_launches_at_five_times_original_muzzle_speed() {
        let mut f = flight();
        let mut c = Combat::default();
        c.start_air(&f, 1, 5);
        c.fight.as_mut().unwrap().gun_held = true;
        c.tick(&mut f, 0.001);
        let round = c
            .fight
            .as_ref()
            .unwrap()
            .rounds
            .iter()
            .find(|r| !r.hostile)
            .unwrap();
        assert!(((round.velocity - f.velocity).length() - 4250.0).abs() < 0.02);
        // A round crosses a complete airframe within a single normal substep.
        // The swept test must still detect the intervening fuselage.
        assert!(bullet_hit(&f, DVec3::new(0., 0., -20.), DVec3::new(0., 0., 20.)).is_some());
    }
    #[test]
    fn cannon_sound_events_match_emitted_rounds_and_stop_without_ammo() {
        let mut f = flight();
        let mut c = Combat::default();
        c.start_air(&f, 1, 5);
        c.fight.as_mut().unwrap().gun = 3;
        c.fight.as_mut().unwrap().gun_held = true;
        for _ in 0..100 {
            c.tick(&mut f, 1. / 120.);
        }
        assert_eq!(c.fight.as_ref().unwrap().gun, 0);
        assert_eq!(
            c.audio_events
                .iter()
                .filter(|e| e.kind == crate::sound::Kind::Cannon && e.own)
                .count(),
            3
        );
    }
    #[test]
    fn paired_countermeasures_respect_independent_inventory_and_cooldown() {
        let inventory = |c: &Combat| {
            let f = c.fight.as_ref().unwrap();
            (f.flares, f.chaff)
        };
        let f = flight();
        let mut c = Combat::default();
        c.start_air(&f, 1, 6);
        assert!(c.deploy_pair(&f));
        assert_eq!(inventory(&c), (23, 23));
        assert!(!c.deploy_pair(&f));
        assert_eq!(inventory(&c), (23, 23));
        c.clock += 0.7;
        c.fight.as_mut().unwrap().flares = 0;
        assert!(c.deploy_pair(&f));
        assert_eq!(inventory(&c), (0, 22));
        let mut paused = f.clone();
        paused.paused = true;
        c.clock += 0.7;
        assert!(!c.deploy_pair(&paused));
        assert_eq!(inventory(&c), (0, 22));
    }
    #[test]
    fn difficulty_changes_pilot_skill_but_not_airframe_or_weapon_power() {
        assert!(ai::skill(Difficulty::Training) < ai::skill(Difficulty::Balanced));
        assert!(ai::skill(Difficulty::Balanced) < ai::skill(Difficulty::Hard));
        assert!(Difficulty::Training.interval() > Difficulty::Hard.interval());
        assert_eq!(Difficulty::Training.damage(), Difficulty::Hard.damage());
    }
    #[test]
    fn opponents_launch_after_warmup_and_do_not_have_infinite_ammo() {
        for scenario in 5..=7 {
            let mut f = flight();
            let mut c = Combat::default();
            c.start_air(&f, 1, scenario);
            for _ in 0..240 {
                c.tick(&mut f, 1.0 / 120.0);
            }
            assert_eq!(c.fight.as_ref().unwrap().launched, 0);
            for _ in 0..7200 {
                f.position += f.velocity * (1.0 / 120.0);
                c.tick(&mut f, 1.0 / 120.0);
            }
            let fight = c.fight.as_ref().unwrap();
            assert!(fight.launched > 0, "scenario {scenario}");
            assert!(fight.launched <= 18);
            assert!(
                c.targets
                    .iter()
                    .all(|t| t.position.is_finite() && t.orientation.is_finite())
            );
        }
    }
    #[test]
    fn countermeasure_bursts_can_divert_but_do_not_guarantee_a_miss() {
        for kind in [Weapon::Heat, Weapon::Radar] {
            let mut diverted = 0;
            for seed in 1..=64 {
                let mut f = flight();
                let mut c = Combat::default();
                c.start_air(&f, 1, 5);
                let position = f.position - f.forward() * 1000.0;
                let velocity = f.forward() * 450.0;
                let mut gnc = super::super::guidance::Gnc::launch(
                    velocity,
                    super::super::guidance::player(&f),
                    seed,
                );
                gnc.active = kind == Weapon::Radar;
                c.fight.as_mut().unwrap().enemy_missiles.push(Missile {
                    gnc,
                    position,
                    velocity,
                    kind,
                    target: 0,
                    age: 1.0,
                    alive: true,
                    guided: true,
                    lost_for: 0.0,
                    trail: vec![position],
                    trail_clock: 0.0,
                    decoy: None,
                });
                let wrong = if kind == Weapon::Heat {
                    Weapon::Radar
                } else {
                    Weapon::Heat
                };
                assert!(c.deploy(&f, wrong));
                c.tick(&mut f, 0.01);
                assert!(c.fight.as_ref().unwrap().enemy_missiles[0].decoy.is_none());
                assert!(c.deploy(&f, kind));
                c.tick(&mut f, 0.01);
                // A freshly ejected flare has not developed its radiance yet.
                assert!(c.fight.as_ref().unwrap().enemy_missiles[0].decoy.is_none());
                for _ in 0..60 {
                    c.tick(&mut f, 0.01);
                }
                diverted += usize::from(c.fight.as_ref().unwrap().diverted > 0);
            }
            assert!(diverted > 0 && diverted < 64, "{kind:?}: {diverted}");
        }
    }
    #[test]
    fn damage_changes_engine_radar_and_individual_surfaces_and_can_destroy() {
        let mut f = flight();
        apply_hit(&mut f, DVec3::new(-3.0, 0.0, 2.8), 0.45, false);
        assert!(f.damage.surfaces[2] < f.damage.surfaces[6]);
        apply_hit(&mut f, DVec3::new(0.0, 0.0, -3.0), 0.5, false);
        assert!(f.damage.radar < 0.25);
        assert!(!f.assist);
        let mut f = flight();
        apply_hit(&mut f, DVec3::new(0.0, 0.0, 4.0), 0.6, true);
        assert!(f.damage.engine < 0.3);
        assert!(!f.crashed);
        apply_hit(&mut f, DVec3::ZERO, 1.0, true);
        assert!(f.crashed);
    }
    #[test]
    fn ai_breaks_when_player_missile_closes() {
        let f = flight();
        let mut c = Combat::default();
        c.start_air(&f, 1, 6);
        let t = c.targets[0].clone();
        c.missiles.push(Missile {
            gnc: super::guidance::Gnc::default(),
            position: t.position - t.velocity.normalize() * 1000.0,
            velocity: t.velocity.normalize() * 450.0,
            kind: Weapon::Radar,
            target: 0,
            age: 1.0,
            alive: true,
            guided: true,
            lost_for: 0.0,
            trail: vec![],
            trail_clock: 0.0,
            decoy: None,
        });
        c.advance(&f, 0.02);
        assert_ne!(c.targets[0].pilot.as_ref().unwrap().mode, PilotMode::Evade);
        c.missiles[0].gnc.active = true;
        c.missiles[0].gnc.gimbal = t.velocity.normalize();
        c.advance(&f, 0.02);
        assert_eq!(c.targets[0].pilot.as_ref().unwrap().mode, PilotMode::Evade);
    }
    #[test]
    fn opponent_dispenses_matching_countermeasure_from_its_own_inventory() {
        let mut f = flight();
        let mut c = Combat::default();
        c.start_air(&f, 1, 6);
        let target = &c.targets[0];
        let contact = super::super::guidance::target(target);
        let velocity = target.velocity.normalize() * 450.0;
        let position = target.position - target.velocity.normalize() * 1000.0;
        c.missiles.push(Missile {
            gnc: super::super::guidance::Gnc::launch(velocity, contact, 1),
            position,
            velocity,
            kind: Weapon::Radar,
            target: 0,
            age: 1.0,
            alive: true,
            guided: true,
            lost_for: 0.0,
            trail: vec![],
            trail_clock: 0.0,
            decoy: None,
        });
        c.tick(&mut f, 0.01);
        assert_eq!(
            c.targets[0].pilot.as_ref().unwrap().countermeasures,
            [24, 23]
        );
        assert_eq!(c.fight.as_ref().unwrap().chaff, 24);
        assert!(
            c.fight
                .as_ref()
                .unwrap()
                .decoys
                .iter()
                .any(|d| d.owner == Some(0) && d.kind == Weapon::Radar)
        );
    }
    #[test]
    fn cannon_sweep_hits_wing_and_leaves_empty_space_beside_nose_clear() {
        let f = flight();
        assert!(bullet_hit(&f, DVec3::new(2.0, -10.0, 1.0), DVec3::new(2.0, 10.0, 1.0)).is_some());
        assert!(bullet_hit(&f, DVec3::new(3.0, -1.0, -5.0), DVec3::new(3.0, 1.0, -5.0)).is_none());
    }
    #[test]
    fn enemy_missile_damages_player_wing_without_always_destroying_it() {
        let mut f = flight();
        let mut c = Combat::default();
        c.start_air(&f, 1, 6);
        let position = f.position + f.orientation * DVec3::new(-6.0, 0.0, 1.8);
        let velocity = f.velocity + f.forward() * 18.0;
        let mut gnc =
            super::super::guidance::Gnc::launch(velocity, super::super::guidance::player(&f), 1);
        gnc.active = true;
        c.fight.as_mut().unwrap().enemy_missiles.push(Missile {
            gnc,
            position,
            velocity,
            kind: Weapon::Radar,
            target: 0,
            age: 1.0,
            alive: true,
            guided: true,
            lost_for: 0.0,
            trail: vec![],
            trail_clock: 0.0,
            decoy: None,
        });
        c.tick(&mut f, 0.01);
        assert!(f.integrity < 1.0 && f.integrity > 0.1);
        assert!(f.damage.wings[0] < f.damage.wings[1]);
        assert!(!f.crashed);
    }
    #[test]
    fn shaped_target_accepts_wing_hit_and_rejects_empty_space() {
        let f = flight();
        let mut c = Combat::default();
        c.start(&f, 1);
        let t = &c.targets[0];
        let wing = t.position + t.orientation * DVec3::new(2.5, 0.0, 1.1);
        let outside = t.position + t.orientation * DVec3::new(3.0, 0.0, -5.0);
        assert!(target_bullet_hit(t, wing - DVec3::Y * 2.0, wing + DVec3::Y * 2.0, 0.0).is_some());
        assert!(
            target_bullet_hit(t, outside - DVec3::Y * 2.0, outside + DVec3::Y * 2.0, 0.0).is_none()
        );
    }
    #[test]
    fn player_cannon_hit_publishes_enemy_component_snapshot() {
        let mut f = flight();
        let mut c = Combat::default();
        c.start_air(&f, 1, 5);
        let t = &c.targets[0];
        let direction = t.orientation * -DVec3::Z;
        let position = t.position - direction * 20.;
        c.fight.as_mut().unwrap().rounds.push(Round {
            position,
            previous: position,
            velocity: t.velocity + direction * CANNON_MUZZLE_SPEED,
            hostile: false,
            age: 0.,
            damage: 0.075,
            alive: true,
        });
        c.tick(&mut f, 0.01);
        assert_eq!(c.strikes, 1);
        let r = c.recent_hit.as_ref().unwrap();
        assert_eq!(r.index, 0);
        assert_eq!(r.integrity, c.targets[0].health);
        assert!(r.damage.engine < 1.);
    }
    #[test]
    fn hostile_cannon_round_hits_shaped_airframe_and_damages_engine() {
        let mut f = flight();
        f.velocity = DVec3::ZERO;
        let mut c = Combat::default();
        c.start_air(&f, 1, 5);
        let position = f.position - f.forward() * 20.0;
        c.fight.as_mut().unwrap().rounds.push(Round {
            position,
            previous: position,
            velocity: f.forward() * 1200.0,
            hostile: true,
            age: 0.0,
            damage: 0.08,
            alive: true,
        });
        c.tick(&mut f, 0.025);
        assert_eq!(c.fight.as_ref().unwrap().gun_hits, 1);
        assert!(f.damage.engine < 1.0);
    }
    #[test]
    fn opponent_can_acquire_and_launch_heat_seeker_from_rear_aspect() {
        let mut f = flight();
        let mut c = Combat::default();
        c.start_air(&f, 1, 5);
        let t = &mut c.targets[0];
        t.position = f.position - f.forward() * 2200.0;
        t.velocity = f.forward() * 260.0;
        t.orientation = f.orientation;
        t.pilot.as_mut().unwrap().launch_at = 0.0;
        for _ in 0..180 {
            c.tick(&mut f, 1.0 / 120.0);
        }
        assert!(
            c.fight
                .as_ref()
                .unwrap()
                .enemy_missiles
                .iter()
                .any(|m| m.kind == Weapon::Heat)
        );
    }
}
