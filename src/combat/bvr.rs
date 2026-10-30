//! Beyond-visual-range game model. All performance numbers are gameplay parameters.
use super::*;
use dogfight::Difficulty;
use guidance::{Contact, Support};
pub mod cockpit_radar;
pub mod setup_screen;
pub mod ui;
pub use ui::{Menu, controls, draw, setup};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RadarMode {
    Search,
    Tws,
    Stt,
}
impl RadarMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Search => "SEARCH",
            Self::Tws => "TWS",
            Self::Stt => "STT",
        }
    }
}
#[derive(Clone, Default)]
pub struct Track {
    pub position: DVec3,
    pub velocity: DVec3,
    pub seen: f64,
    pub confidence: f64,
    pub reports: u32,
    pub friendly_report: bool,
}
impl Track {
    pub fn valid(&self, now: f64) -> bool {
        self.confidence > 0.15 && now - self.seen < 18.
    }
}
#[derive(Clone)]
pub struct Radar {
    pub powered: bool,
    pub mode: RadarMode,
    pub range: usize,
    pub elevation: f64,
    pub scan: f64,
    pub tracks: Vec<Track>,
    next: f64,
}
impl Default for Radar {
    fn default() -> Self {
        Self {
            powered: true,
            mode: RadarMode::Tws,
            range: 2,
            elevation: 0.,
            scan: 0.,
            tracks: vec![],
            next: 0.,
        }
    }
}
pub fn sense_local(c: &mut Combat, f: &Flight) {
    let r = &mut c.radar;
    r.scan = ((c.clock * 32.).rem_euclid(240.) - 120.).abs() - 60.;
    r.tracks.resize_with(c.targets.len(), Track::default);
    if c.clock < r.next {
        return;
    }
    r.next = c.clock + 0.2;
    for (i, t) in c.targets.iter().enumerate() {
        let tr = &mut r.tracks[i];
        tr.confidence = (tr.confidence - 0.015).max(0.);
        let offset = t.position - f.position;
        let local = f.orientation.conjugate() * offset;
        let az = local.x.atan2(-local.z).to_degrees();
        let el = local.y.atan2(local.x.hypot(local.z)).to_degrees();
        let beam = match r.mode {
            RadarMode::Stt => i == c.selected && az.abs() < 60.,
            RadarMode::Tws if tr.valid(c.clock) && c.clock - tr.seen > 1. => az.abs() < 60.,
            _ => (az - r.scan).abs() < 8.,
        };
        if t.alive
            && r.powered
            && f.battery
            && f.damage.radar > 0.25
            && offset.length() < LOCAL_RANGES[r.range]
            && (el - r.elevation).abs() < 12.
            && beam
            && radar_signal(f.position, guidance::target(t)) > 0.18
        {
            *tr = Track {
                position: t.position,
                velocity: t.velocity,
                seen: c.clock,
                confidence: (tr.confidence + 0.5).min(1.),
                reports: tr.reports + 1,
                friendly_report: false,
            };
        }
    }
}
#[derive(Clone, Copy)]
pub struct Shot {
    pub launcher: Option<usize>,
    pub target: Option<usize>,
    pub friendly: bool,
}
#[derive(Clone, Default)]
struct Ai {
    memory: Option<(Option<usize>, DVec3, DVec3, f64)>,
    launch: f64,
    ammo: u8,
    decoys: u8,
    decoy_at: f64,
}
pub struct Battle {
    pub enemies: usize,
    pub allies: usize,
    pub defense: bool,
    pub difficulty: Difficulty,
    pub radar: Radar,
    pub shots: Vec<Shot>,
    ai: Vec<Ai>,
    pub objective: DVec3,
    pub result: &'static str,
    pub range_at_start: f64,
    work: WorkBuffers,
}
#[derive(Default)]
struct WorkBuffers {
    old: Vec<(DVec3, DQuat)>,
    neighbours: Vec<(bool, Contact)>,
    decisions: Vec<Option<FlightDecision>>,
    requests: Vec<(usize, Option<usize>)>,
}
pub const RANGES: [f64; 4] = [40000., 80000., 160000., 240000.];
pub const LOCAL_RANGES: [f64; 4] = [5000., 10000., 20000., 40000.];
pub const FORMATIONS: [(usize, usize); 4] = [(2, 0), (2, 1), (4, 1), (8, 3)];
pub fn flight() -> Flight {
    let mut f = super::flight();
    f.position = surface_point(100000., 0., 11000.);
    f.orientation = facing(-DVec3::Z, f.up());
    f.velocity = f.forward() * 290.;
    f.spool = 0.88;
    f.throttle = 0.88;
    f.fuel = 3200.;
    let (t, p, r, s) = atmosphere(f.altitude());
    f.temperature = t;
    f.pressure = p;
    f.density = r;
    f.airspeed = 290.;
    f.mach = 290. / s;
    f.thrust = crate::propulsion::jet_output(f.spool, r, f.mach).0;
    f.status = "BVR / M TACTICAL MAP · TAB TRACK · SPACE LAUNCH".into();
    f
}
pub fn radar_signal(observer: DVec3, c: Contact) -> f64 {
    if !c.alive || !guidance::terrain_visible(observer, c.position) {
        return 0.;
    }
    (c.strength * (170000. / observer.distance(c.position).max(1.)).powi(4)).min(1.)
        * guidance::radar_notch(observer, c)
}
pub fn envelope(position: DVec3, velocity: DVec3, c: Contact) -> (f64, f64) {
    let los = (c.position - position).normalize_or_zero();
    let closure = (velocity - c.velocity).dot(los);
    let max = (30000.
        + (position.length() - RADIUS).max(0.) * 3.
        + velocity.length() * 40.
        + closure * 35.)
        .clamp(28000., 115000.);
    (max, max * 0.48)
}
pub fn start(c: &mut Combat, f: &Flight, generation: u64, m: &Menu) {
    c.start_air(f, generation, 6);
    c.targets.truncate(1);
    let template = c.targets[0].clone();
    c.targets.clear();
    let (enemies, allies) = FORMATIONS[m.formation.min(3)];
    let distance = 125000.;
    let right = f.orientation * DVec3::X;
    for i in 0..enemies + allies {
        let mut t = template.clone();
        t.brain.id = i;
        t.brain.long_range = true;
        let friend = i >= enemies;
        let offset = if friend {
            right * ((i - enemies + 1) as f64 * 1800.) - f.forward() * 1200.
        } else {
            f.forward() * (distance + (i / 4) as f64 * 6000.)
                + right * ((i % 4) as f64 - 1.5) * 7000.
        };
        t.position = (f.position + offset).normalize() * (RADIUS + 11000. + (i % 3) as f64 * 500.);
        let dir = if friend { f.forward() } else { -f.forward() };
        t.velocity = dir * 290.;
        t.orientation = facing(dir, t.position.normalize());
        t.fuel = 3200.;
        t.speed = 290.;
        t.throttle = 0.88;
        t.damage.rng = generation
            .wrapping_mul(997)
            .wrapping_add(i as u64 * 131 + 3);
        ai::initialize(&mut t);
        c.targets.push(t);
    }
    // Preserve the Gripen's existing BVR loadout; other aircraft retain their
    // aircraft-specific inventory initialized by start_air.
    if f.aircraft == crate::aircraft::Aircraft::Gripen {
        c.ammo = [2, 4];
    }
    c.weapon = Weapon::Radar;
    c.fight.as_mut().unwrap().difficulty = m.difficulty;
    c.bvr = Some(Battle {
        enemies,
        allies,
        defense: m.defense,
        difficulty: m.difficulty,
        radar: Radar {
            powered: true,
            mode: RadarMode::Tws,
            range: 2,
            elevation: 0.,
            scan: 0.,
            tracks: vec![Track::default(); enemies + allies],
            next: 0.,
        },
        shots: vec![],
        ai: vec![
            Ai {
                ammo: 4,
                decoys: 24,
                ..default()
            };
            enemies + allies
        ],
        objective: surface_point(100000., 45000., terrain_height(100000., 45000.) + 25.),
        result: "ENGAGEMENT ACTIVE",
        range_at_start: distance,
        work: WorkBuffers::default(),
    });
    c.notify(format!(
        "BVR {} vs {} / {} / {}",
        allies + 1,
        enemies,
        m.difficulty.label(),
        if m.defense {
            "DEFEND HOME SECTOR"
        } else {
            "ELIMINATION"
        }
    ));
}
impl Battle {
    pub fn rwr_tracking(&self, c: &Combat, index: usize) -> bool {
        self.ai.get(index).is_some_and(|a| {
            a.memory
                .is_some_and(|m| m.0.is_none() && c.clock - m.3 < 1.)
        }) || self.shots.iter().zip(&c.missiles).any(|(s, m)| {
            s.launcher == Some(index) && s.target.is_none() && m.alive && !m.gnc.active
        })
    }
    pub fn tracking_player(&self, c: &Combat) -> usize {
        self.ai
            .iter()
            .enumerate()
            .take(self.enemies)
            .filter(|(i, a)| {
                c.targets[*i].alive
                    && c.targets[*i].damage.radar > 0.25
                    && a.memory
                        .is_some_and(|m| m.0.is_none() && c.clock - m.3 < 1.)
            })
            .count()
    }
    pub fn friendly(&self, i: usize) -> bool {
        i >= self.enemies
    }
    pub fn cue(&self, c: &Combat, f: &Flight) -> Result<(), &'static str> {
        if f.aircraft==crate::aircraft::Aircraft::Hornet&&f.systems.hornet_wing_fold>0.02 {return Err("UNFOLD WINGS");}
        if f.crashed {
            return Err("AIRFRAME LOST");
        }
        if !f.battery {
            return Err("POWER OFF");
        }
        let Some(t) = c.targets.get(c.selected).filter(|t| t.alive) else {
            return Err("NO TARGET");
        };
        if self.friendly(c.selected) {
            return Err("FRIENDLY / INHIBIT");
        }
        let range = f.position.distance(t.position);
        if c.weapon == Weapon::Heat {
            if range > c.weapon.player_range(f) {
                return Err("IR OUT OF RANGE");
            }
            if f.forward().dot((t.position - f.position).normalize()) < c.weapon.player_cone(f).to_radians().cos() {
                return Err("POINT NOSE AT TARGET");
            }
            if guidance::quality(Weapon::Heat, f.position, guidance::target(t), c.sun).0 < 0.18 {
                return Err("IR SEEKER NO TRACK");
            }
        } else {
            if !self.radar.powered {
                return Err("RADAR STANDBY");
            }
            if f.damage.radar < 0.25 {
                return Err("RADAR FAILED");
            }
            if self.radar.mode == RadarMode::Search {
                return Err("SELECT TWS OR STT");
            }
            let tr = &self.radar.tracks[c.selected];
            if !tr.valid(c.clock) || c.clock - tr.seen > 3. {
                return Err("NO FRESH RADAR TRACK");
            }
            if f.forward().dot((tr.position - f.position).normalize()) < 60_f64.to_radians().cos() {
                return Err("RADAR GIMBAL LIMIT");
            }
            if range > envelope(f.position, f.velocity, guidance::target(t)).0 {
                return Err("OUTSIDE ESTIMATED RMAX");
            }
        }
        if range < 500. {
            return Err("TOO CLOSE");
        }
        Ok(())
    }
}
fn launch(
    c: &mut Combat,
    b: &mut Battle,
    f: &Flight,
    launcher: Option<usize>,
    target: Option<usize>,
    kind: Weapon,
) {
    // Retain live references and camera IDs; compact only between scenarios.
    if c.missiles.len() >= 64 {
        c.notify("MISSILE POOL FULL");
        return;
    }
    let (p, v, q, friendly) = launcher.map_or((f.position, f.velocity, f.orientation, true), |i| {
        let t = &c.targets[i];
        (t.position, t.velocity, t.orientation, b.friendly(i))
    });
    let contact = target.map_or_else(|| guidance::player(f), |i| guidance::target(&c.targets[i]));
    let station=if launcher.is_none() && f.aircraft!=crate::aircraft::Aircraft::Gripen {crate::aircraft::launch_station(f,kind,c.ammo)}else{DVec3::new(
        if c.missiles.len() % 2 == 0 { -2.5 } else { 2.5 },
        -0.5,
        0.7,
    )};
    let position = p + q * station;
    let rail=if launcher.is_none() && f.aircraft==crate::aircraft::Aircraft::F22 && kind==Weapon::Heat {
        let i=f.aircraft.spec().ammo[0].saturating_sub(c.ammo[0]) as usize;
        crate::aircraft::bays::missile_pose(i.min(1),f.systems.side_bays[i.min(1)].launcher).rotation.as_dquat()
    }else{DQuat::IDENTITY};
    let main=launcher.is_none()&&kind==Weapon::Radar&&c.internal_main_slot(f).is_some();
    let rate=q*launcher.map_or(f.rates,|i|c.targets[i].dynamics.rates);
    let inherited=v+rate.cross(q*station);
    let velocity = if main{inherited-q*DVec3::Y*8.}else{inherited + q * (rail * -DVec3::Z * 18. - DVec3::Y * if launcher.is_none() && kind==Weapon::Heat && matches!(f.aircraft,crate::aircraft::Aircraft::F22|crate::aircraft::Aircraft::Hornet) {0.}else{4.})};
    let mut gnc = guidance::Gnc::launch(
        velocity,
        contact,
        c.generation.wrapping_mul(997) + c.missiles.len() as u64 + 1,
    );
    gnc.long_range = kind == Weapon::Radar;
    if launcher.is_none() { gnc = gnc.for_aircraft(f.aircraft, kind); }
    gnc=gnc.inherit(q*rail,rate);
    c.missiles.push(Missile {
        gnc,
        position,
        velocity,
        kind,
        target: target.unwrap_or(0),
        age: 0.,
        alive: true,
        guided: true,
        lost_for: 0.,
        decoy: None,
        trail: vec![position],
        trail_clock: 0.,
    });
    b.shots.push(Shot {
        launcher,
        target,
        friendly,
    });
    crate::sound::emit(
        &mut c.audio_events,
        crate::sound::Kind::Launch,
        position,
        v,
        launcher.is_none(),
    );
}
pub fn fire(c: &mut Combat, f: &Flight) -> bool {
    if f.paused {
        c.notify("RESUME TO LAUNCH");
        return false;
    }
    if let Err(reason) = c.cue(f) {
        c.notify(reason);
        return false;
    }
    if c.lock < c.weapon.lock_time() {
        c.notify("WAIT FOR TRACK QUALITY");
        return false;
    }
    if c.ammo[c.weapon.index()] == 0 {
        c.notify("NO MISSILES OF THIS TYPE");
        return false;
    }
    if c.clock - c.last_launch < 0.6 || c.missiles.len() >= 64 {
        return false;
    }
    if f.aircraft==crate::aircraft::Aircraft::F22 && c.weapon==Weapon::Heat && f.gear_extension>0.05 {c.notify("RETRACT LANDING GEAR");return false;}
    if let Some(accepted)=c.queue_main_launch(f) {return accepted;}
    if let Some(accepted)=c.queue_side_launch(f) {return accepted;}
    let mut b = c.bvr.take().unwrap();
    launch(c, &mut b, f, None, Some(c.selected), c.weapon);
    c.bvr = Some(b);
    c.ammo[c.weapon.index()] -= 1;
    c.fired += 1;
    c.last_launch = c.clock;
    c.notify("MISSILE AWAY / SUPPORT UNTIL ACTIVE");
    true
}
fn sense(b: &mut Battle, c: &Combat, f: &Flight, dt: f64) {
    b.radar.scan = ((c.clock * 32.).rem_euclid(240.) - 120.).abs() - 60.;
    if c.clock < b.radar.next {
        return;
    }
    b.radar.next = c.clock + 0.2;
    for (i, t) in c.targets.iter().enumerate() {
        let track = &mut b.radar.tracks[i];
        track.confidence = (track.confidence - 0.015).max(0.);
        if !t.alive {
            continue;
        }
        let r = t.position - f.position;
        let local = f.orientation.conjugate() * r;
        let az = local.x.atan2(-local.z).to_degrees();
        let el = local.y.atan2(local.x.hypot(local.z)).to_degrees();
        let beam = if b.radar.mode == RadarMode::Stt {
            i == c.selected && az.abs() < 60.
        } else if b.radar.mode == RadarMode::Tws
            && track.valid(c.clock)
            && c.clock - track.seen > 1.0
        {
            az.abs() < 60.
        } else {
            (az - b.radar.scan).abs() < 8.
        };
        let own = b.radar.powered
            && f.battery
            && f.damage.radar > 0.25
            && r.length() < RANGES[b.radar.range]
            && (el - b.radar.elevation).abs() < 12.
            && beam
            && radar_signal(f.position, guidance::target(t)) > 0.18;
        let shared = c.targets.iter().enumerate().skip(b.enemies).any(|(j, a)| {
            j != i
                && a.alive
                && a.damage.radar > 0.25
                && a.position.distance(t.position) < 160000.
                && (a.orientation * -DVec3::Z).dot((t.position - a.position).normalize())
                    > 65_f64.to_radians().cos()
                && radar_signal(a.position, guidance::target(t)) > 0.18
        });
        if own || shared || i >= b.enemies {
            track.position = t.position;
            track.velocity = t.velocity;
            track.seen = c.clock;
            track.confidence = (track.confidence + 0.5).min(1.);
            track.reports += 1;
            track.friendly_report = !own && shared;
        }
    }
    let _ = dt;
}
#[derive(Clone, Copy)]
struct FlightDecision {
    desired: DVec3,
    speed: f64,
    dir: DVec3,
    memory: Option<(Option<usize>, DVec3, DVec3, f64)>,
    incoming: bool,
}
fn advance_aircraft(
    targets: &mut [Target],
    decisions: &[Option<FlightDecision>],
    environment: &Environment,
    difficulty: Difficulty,
    dt: f64,
    parallel: bool,
) {
    let run = |targets: &mut [Target], decisions: &[Option<FlightDecision>]| {
        for (t, decision) in targets.iter_mut().zip(decisions) {
            if let Some(plan) = decision {
                ai::fly(t, environment, plan.desired, plan.speed, difficulty, dt);
            } else {
                t.advance(dt, environment);
            }
        }
    };
    // Small formations remain serial: scheduling costs more than it saves.
    // Batches borrow disjoint aircraft; no shared RNG, sensors or weapons mutate.
    if parallel && targets.len() >= 8 {
        use bevy::tasks::{ComputeTaskPool, TaskPoolBuilder};
        let pool = ComputeTaskPool::get_or_init(|| {
            TaskPoolBuilder::new()
                .num_threads(
                    crate::performance::compute_threads(),
                )
                .thread_name("Flight compute".into())
                .build()
        });
        if pool.thread_num() > 1 {
            let chunk = targets.len().div_ceil(pool.thread_num().min(4));
            pool.scope(|scope| {
                for (targets, decisions) in targets.chunks_mut(chunk).zip(decisions.chunks(chunk)) {
                    scope.spawn(async {
                        run(targets, decisions);
                    });
                }
            });
            return;
        }
    }
    run(targets, decisions);
}
pub fn advance(c: &mut Combat, f: &mut Flight, dt: f64) {
    let mut b = c.bvr.take().unwrap();
    let mut work = std::mem::take(&mut b.work);
    c.clock += dt;
    work.old.clear();
    work.old
        .extend(c.targets.iter().map(|t| (t.position, t.orientation)));
    let old = &work.old;
    sense(&mut b, c, f, dt);
    // Every aircraft evaluates sensors and pilot decisions on every simulation tick.
    for i in 0..c.targets.len() {
        let friend = b.friendly(i);
        let t = &c.targets[i];
        if !t.alive {
            continue;
        }
        {
            // Rebuild from the current sequential memories, not a stale tick
            // snapshot: earlier pilots may already have reassigned this step.
            let mut assignments=[0usize;12];
            for (j,p) in b.ai.iter().enumerate(){
                if j!=i && b.friendly(j)==friend {
                    if let Some(m)=p.memory.filter(|m|c.clock-m.3<3. && m.0.is_none_or(|id|id<c.targets.len())) {
                        assignments[m.0.unwrap_or(c.targets.len())]+=1;
                    }
                }
            }
            let candidates = c
                .targets
                .iter()
                .enumerate()
                .filter(|(j, t)| t.alive && b.friendly(*j) != friend)
                .map(|(j, t)| (Some(j), guidance::target(t)))
                .chain((!friend && !f.crashed).then(|| (None, guidance::player(f))));
            let seen = candidates
                .into_iter()
                .filter(|(_, a)| ai::visible(t, *a, c.sun))
                .map(|candidate| {
                        let assigned=assignments[candidate.0.unwrap_or(c.targets.len())];
                        let score=t.position.distance_squared(candidate.1.position)
                            * (1.
                                + assigned as f64
                                    * if b.difficulty == Difficulty::Hard {
                                        0.65
                                    } else {
                                        0.08
                                    });
                        (score,candidate)
                })
                .min_by(|a,z|a.0.total_cmp(&z.0)).map(|(_,candidate)|candidate);
            if let Some((id, a)) = seen {
                let lag = 0.12 + (1. - ai::skill(b.difficulty)) * 0.8;
                let noise = t.position.normalize()
                    * ((3. + (1. - ai::skill(b.difficulty)) * 30.)
                        * (c.clock * 0.37 + i as f64).sin());
                b.ai[i].memory = Some((
                    id,
                    a.position - a.velocity * lag + noise,
                    a.velocity,
                    c.clock,
                ));
            }
        }
    }
    work.neighbours.clear();
    work.neighbours.extend(
        c.targets
            .iter()
            .enumerate()
            .filter(|(_, t)| t.alive)
            .map(|(i, t)| (b.friendly(i), guidance::target(t)))
            .chain((!f.crashed).then_some((true, guidance::player(f)))),
    );
    let neighbours = &work.neighbours;
    let decisions = &mut work.decisions;
    decisions.clear();
    for i in 0..c.targets.len() {
        let friend = b.friendly(i);
        let partner = if i % 2 == 0 { i + 1 } else { i - 1 };
        let shared =
            b.ai.get(partner)
                .filter(|_| b.friendly(partner) == friend)
                .and_then(|p| p.memory)
                .filter(|m| c.clock - m.3 < 3.);
        let memory = b.ai[i].memory.or(shared);
        let t = &mut c.targets[i];
        t.brain.memory = memory.map(|(_, p, v, at)| (p, v, at));
        if !t.alive {
            decisions.push(None);
            continue;
        }
        let up = t.position.normalize();
        let dir = t.velocity.normalize_or(-DVec3::Z);
        let right = dir.cross(up).normalize_or(DVec3::X);
        let incoming = c
            .missiles
            .iter()
            .zip(&b.shots)
            .filter(|(m, s)| {
                // Cheap side and distance filters before the line-of-sight warning.
                s.friendly != friend
                    && m.position.distance(t.position)
                        < if b.difficulty == Difficulty::Training {
                            12000.
                        } else {
                            26000.
                        }
                    && ai::warning(t, m, c.sun)
            })
            .min_by(|a, z| {
                a.0.position
                    .distance_squared(t.position)
                    .total_cmp(&z.0.position.distance_squared(t.position))
            });
        let supporting = c
            .missiles
            .iter()
            .zip(&b.shots)
            .any(|(m, s)| m.alive && !m.gnc.active && s.launcher == Some(i));
        let goal = memory.filter(|m| c.clock - m.3 < 20.).map_or(
            if friend {
                f.position + f.forward() * 12000.
            } else {
                b.objective
            },
            |(_, p, v, at)| p + v * (c.clock - at),
        );
        let mut desired = (goal - t.position).normalize_or(dir);
        if memory.is_none() {
            if friend {
                let station = f.position - f.forward() * 1800.
                    + (f.orientation * DVec3::X) * ((i - b.enemies + 1) as f64 * 800.);
                desired = (station - t.position).normalize_or(dir);
            } else if let Some(bearing) = ai::rf_bearing(t, f, b.radar.powered) {
                desired = bearing;
            }
        }
        // Attack elements press the defended sector; escorts engage interceptors.
        if b.defense && !friend && i % 2 == 0 {
            desired = (b.objective.normalize() * (RADIUS + 10000.) - t.position).normalize_or(dir);
        }
        let mut dragging = false;
        if let Some((missile, _)) = incoming {
            let los = (missile.position - t.position).normalize_or(dir);
            let beam = los.cross(up).normalize_or(right) * if i % 2 == 0 { 1. } else { -1. };
            dragging = missile.position.distance(t.position) > 18000.;
            desired = if dragging {
                (-los + beam * 0.35).normalize()
            } else {
                (beam + up * 0.03).normalize()
            };
        } else if supporting {
            desired = (desired + right * if i % 2 == 0 { 0.65 } else { -0.65 }).normalize();
        }
        let clearance = t.position.length() - RADIUS - terrain_height(t.position.x, t.position.z);
        if clearance < 900. {
            desired = (dir + up).normalize();
        }
        // Pair tactics: a supporting shooter cranks; an available wingman
        // presses an offset intercept. Flight control still obeys the same plant.
        if incoming.is_none() && !supporting && i % 2 == 1 {
            desired = (desired + right * 0.18).normalize_or(dir);
        }
        let obstacles = neighbours
            .iter()
            // Avoidance ignores aircraft beyond this radius. Reject them before
            // terrain/radar tests, preserving full-rate decisions and physics.
            .filter(|(_, contact)| contact.position.distance(t.position) < 2200.)
            .filter(|(team, contact)| *team == friend || ai::visible(t, *contact, c.sun))
            .map(|(_, contact)| (contact.position, contact.velocity));
        desired = (desired + ai::avoidance_iter(t.position, t.velocity, obstacles) * 1.6)
            .normalize_or(dir);
        t.brain.mode = if dragging {
            "DRAG / EXTEND"
        } else if incoming.is_some() {
            "NOTCH / DEFEND"
        } else if supporting {
            "CRANK / SUPPORT"
        } else if memory.is_some() {
            "INTERCEPT / WINGMAN"
        } else {
            "SEARCH / FORMATION"
        };
        t.brain.elapsed = c.clock;
        t.brain.observed = memory.is_some_and(|m| c.clock - m.3 < 0.1);
        let speed = if incoming.is_some() { 330. } else { 300. };
        decisions.push(Some(FlightDecision {
            desired,
            speed,
            dir,
            memory,
            incoming: incoming.is_some(),
        }));
    }
    advance_aircraft(
        &mut c.targets,
        &decisions,
        &c.environment,
        b.difficulty,
        dt,
        true,
    );
    // Commit launch requests and dispenser events in the same aircraft order as
    // the serial loop. Each decision still uses the same pre-integration state.
    let requests = &mut work.requests;
    requests.clear();
    for (i, decision) in decisions.iter().enumerate() {
        let Some(plan) = decision else { continue };
        let FlightDecision {
            dir,
            memory,
            incoming,
            ..
        } = *plan;
        let t = &c.targets[i];
        if !t.alive {
            continue;
        }
        if let Some((target, p, v, seen)) = memory {
            let contact = Contact {
                position: p,
                velocity: v,
                forward: -v.normalize(),
                throttle: 0.9,
                engine: true,
                alive: true,
                decoy: false,
                strength: 1.,
            };
            let reaction = match b.difficulty {
                Difficulty::Training => 18.,
                Difficulty::Balanced => 8.,
                Difficulty::Hard => 4.,
            };
            let interval = match b.difficulty {
                Difficulty::Training => 24.,
                Difficulty::Balanced => 14.,
                Difficulty::Hard => 9.,
            };
            let range = t.position.distance(p);
            let rmax = envelope(t.position, t.velocity, contact).0;
            let fresh = c.clock - seen < 1.;
            let in_flight = c
                .missiles
                .iter()
                .zip(&b.shots)
                .filter(|(m, s)| m.alive && s.launcher == Some(i))
                .count();
            if fresh
                && c.clock > reaction + i as f64 * 0.7
                && c.clock > b.ai[i].launch
                && b.ai[i].ammo > 0
                && in_flight < 2
                && range
                    < rmax
                        * if b.difficulty == Difficulty::Training {
                            0.65
                        } else {
                            0.9
                        }
                && range > 700.
                && !incoming
                && t.damage.radar > 0.25
                && dir.dot((p - t.position).normalize()) > 50_f64.to_radians().cos()
            {
                requests.push((i, target));
                b.ai[i].ammo -= 1;
                b.ai[i].launch = c.clock + interval;
            }
        }
        if incoming && b.ai[i].decoys > 0 && c.clock > b.ai[i].decoy_at {
            b.ai[i].decoys -= 1;
            b.ai[i].decoy_at = c.clock + 1.2;
            if let Some(fight) = &mut c.fight {
                let id = fight.next_decoy;
                fight.next_decoy += 1;
                fight.decoys.push(dogfight::Decoy {
                    owner: Some(i),
                    id,
                    kind: Weapon::Radar,
                    position: t.position,
                    velocity: t.velocity - dir * 35.,
                    age: 0.,
                });
            }
        }
    }
    for (launcher, target) in requests.drain(..) {
        launch(c, &mut b, f, Some(launcher), target, Weapon::Radar);
    }
    if let Some(fight) = &mut c.fight {
        for d in &mut fight.decoys {
            d.age += dt;
            d.velocity *= (-dt * 1.2).exp();
            d.position += d.velocity * dt;
        }
        fight.decoys.retain(|d| d.age < 7.);
    }
    for index in 0..c.missiles.len() {
        let shot = b.shots[index];
        let contact = shot
            .target
            .map_or_else(|| guidance::player(f), |i| guidance::target(&c.targets[i]));
        let (p, forward, available) = shot.launcher.map_or(
            (
                f.position,
                f.forward(),
                !f.crashed
                    && f.battery
                    && b.radar.powered
                    && f.damage.radar > 0.25
                    && b.radar.mode != RadarMode::Search
                    && shot.target.is_some_and(|i| {
                        b.radar.tracks[i].valid(c.clock) && c.clock - b.radar.tracks[i].seen < 3.
                    })
                    && (b.radar.mode != RadarMode::Stt || shot.target == Some(c.selected)),
            ),
            |i| {
                let t = &c.targets[i];
                (
                    t.position,
                    t.orientation * -DVec3::Z,
                    t.alive
                        && t.damage.radar > 0.25
                        && b.ai[i]
                            .memory
                            .is_some_and(|m| m.0 == shot.target && c.clock - m.3 < 1.),
                )
            },
        );
        let m = &mut c.missiles[index];
        if !m.alive {
            continue;
        }
        let before = m.position;
        let mut tracked = contact;
        if let Some(fight) = &c.fight {
            let kind = m.kind;
            for d in fight
                .decoys
                .iter()
                .filter(|d| d.owner == shot.target && d.kind == kind)
            {
                if guidance::consider_decoy(
                    m,
                    contact,
                    dogfight::decoy_contact(d),
                    d.age,
                    d.id,
                    c.sun,
                ) {
                    m.decoy = Some(d.id);
                }
            }
            if let Some(d) = m
                .decoy
                .and_then(|id| fight.decoys.iter().find(|d| d.id == id))
            {
                if guidance::retain_decoy(m, contact, dogfight::decoy_contact(d), d.age, c.sun) {
                    tracked = dogfight::decoy_contact(d);
                } else {
                    m.decoy = None;
                }
            }
        }
        m.gnc.wind = c.environment.wind(m.position);
        guidance::fly(
            m,
            tracked,
            Support {
                position: p,
                forward,
                available,
            },
            c.sun,
            dt,
        );
            if let Some(hit)=crate::carrier::collider::sweep(f,before,m.position,dt,0.12) {
                m.position=hit.point;m.alive=false;
                c.impacts.push(Impact{position:hit.point,age:0.,hit:false});
                continue;
            }
        let target_old = shot
            .target
            .map_or(f.position - f.velocity * dt, |i| old[i].0);
        let miss = damage::closest_relative(before - target_old, m.position - contact.position).0;
        let rotation = shot
            .target
            .map_or(f.orientation, |i| c.targets[i].orientation);
        let body = rotation.conjugate() * miss;
        if contact.alive
            && m.age > 0.4
            && miss.length() < m.kind.fuse() + 9.
            && crate::airframe::distance_to_aircraft(body,if shot.target.is_some(){crate::aircraft::Aircraft::Gripen}else{f.aircraft}) < m.kind.fuse()
        {
            m.alive = false;
            if let Some(i) = shot.target {
                let t = &mut c.targets[i];
                let angles = crate::control_surfaces::angles(
                    t.dynamics.surface,
                    t.dynamics.flaps,
                    t.dynamics.airbrake,
                );
                if b.friendly(i) {
                    damage::missile(&mut t.damage, &mut t.health, body, m.kind, angles);
                } else {
                    damage::enemy_missile(&mut t.damage, &mut t.health, body, m.kind, angles);
                }
                if damage::fatal(&t.damage, t.health) {
                    t.alive = false;
                    t.wreck_age = 0.;
                }
                if !b.friendly(i) && shot.launcher.is_none() {
                    c.recent_hit = Some(super::effects::HitReport::new(i, t, c.clock));
                }
            } else {
                let angles = crate::aircraft::angles(f);
                damage::missile(&mut f.damage, &mut f.integrity, body, m.kind, angles);
                dogfight::finalize_player_damage(f);
            }
            c.impacts.push(Impact {
                position: contact.position + miss,
                age: 0.,
                hit: true,
            });
            c.strikes += 1;
        } else if m.age > (if m.gnc.long_range { 210. } else { 45. })
            || m.position.length() - RADIUS < terrain_height(m.position.x, m.position.z)
            || !contact.alive
        {
            m.alive = false;
        }
    }
    cannon(c, f, dt, b.enemies);
    for impact in &mut c.impacts {
        impact.age += dt;
    }
    c.impacts.retain(|p| p.age < 4.);
    // Resolve shaped contacts for every aircraft, including friendly formations.
    super::contact::advance(c, f, &old, dt);
    c.hits = c.targets[..b.enemies].iter().filter(|t| !t.alive).count();
    if b.result == "ENGAGEMENT ACTIVE" {
        if f.crashed {
            b.result = "MISSION FAILED / PLAYER LOST";
        } else if c.targets[..b.enemies].iter().all(|t| !t.alive) {
            b.result = "MISSION COMPLETE / HOSTILES ELIMINATED";
        } else if b.defense
            && c.targets[..b.enemies]
                .iter()
                .any(|t| t.alive && t.position.distance(b.objective) < 18000.)
        {
            b.result = "MISSION FAILED / DEFENSE LINE BREACHED";
        } else if b.defense && c.clock >= 1200. {
            b.result = "MISSION COMPLETE / SECTOR HELD";
        }
    }
    b.work = work;
    c.bvr = Some(b);
    if c.cue(f).is_ok() {
        c.lock = (c.lock + dt).min(c.weapon.lock_time());
    } else {
        c.lock = 0.;
    }
}

fn cannon(c: &mut Combat, f: &Flight, dt: f64, enemies: usize) {
    let Some(fight) = &mut c.fight else {
        return;
    };
    if fight.gun_held && !f.crashed && f.battery && fight.gun > 0 && c.clock >= fight.gun_at {
        let position = f.position + f.orientation * crate::aircraft::gun_origin(f.aircraft);
        fight.rounds.push(dogfight::Round {
            position,
            previous: position,
            velocity: f.velocity + f.forward() * CANNON_MUZZLE_SPEED,
            hostile: false,
            age: 0.,
            damage: 0.075,
            alive: true,
        });
        fight.gun -= 1;
        fight.gun_at = c.clock + f.aircraft.spec().gun_interval;
        crate::sound::emit(
            &mut c.audio_events,
            crate::sound::Kind::Cannon,
            position,
            f.velocity,
            true,
        );
    }
    for r in &mut fight.rounds {
        r.previous = r.position;
        r.age += dt;
        r.velocity -= r.position.normalize() * 9.81 * dt;
        r.position += r.velocity * dt;
        if !r.alive {
            continue;
        }
        if let Some(hit)=crate::carrier::collider::sweep(f,r.previous,r.position,dt,0.02) {
            r.position=hit.point;r.alive=false;continue;
        }
        for (index, t) in c.targets.iter_mut().enumerate().filter(|(_, t)| t.alive) {
            if let Some(body) = dogfight::target_bullet_hit(t, r.previous, r.position, dt) {
                if index < enemies {
                    damage::enemy_projectile(&mut t.damage, &mut t.health, body, r.damage);
                } else {
                    damage::projectile(&mut t.damage, &mut t.health, body, r.damage);
                }
                r.alive = false;
                if damage::fatal(&t.damage, t.health) {
                    t.alive = false;
                    t.wreck_age = 0.;
                }
                if index < enemies {
                    c.recent_hit = Some(super::effects::HitReport::new(index, t, c.clock));
                }
                break;
            }
        }
    }
    fight.rounds.retain(|r| r.alive && r.age < 2.5);
}

/// Headless tactical/weapon diagnostics; the player uses an explicit kinematic
/// intercept path here, rather than claiming full six-degree-of-freedom flight.
pub fn study() {
    let mut runs = Vec::new();
    for formation in 0..4 {
        for defense in [false, true] {
            for difficulty in [Difficulty::Training, Difficulty::Balanced, Difficulty::Hard] {
                let mut f = flight();
                let mut c = Combat::default();
                let m = Menu {
                    formation,
                    defense,
                    difficulty,
                    ..default()
                };
                start(&mut c, &f, 77, &m);
                let mut first_launch = None;
                let mut active = 0;
                let mut finite = true;
                for _ in 0..10000 {
                    let dt = 0.05;
                    f.position += f.velocity * dt;
                    f.elapsed += dt;
                    c.tick(&mut f, dt);
                    if first_launch.is_none() && c.missiles.iter().any(|m| m.alive) {
                        first_launch = Some(c.clock);
                    }
                    active = active.max(c.missiles.iter().filter(|m| m.gnc.active).count());
                    finite &= c
                        .targets
                        .iter()
                        .all(|t| t.position.is_finite() && t.velocity.is_finite())
                        && c.missiles
                            .iter()
                            .all(|m| m.position.is_finite() && m.velocity.is_finite());
                    if c.cue(&f).is_ok() && c.lock >= c.weapon.lock_time() {
                        c.fire(&f);
                    }
                    if f.crashed || c.bvr.as_ref().unwrap().result != "ENGAGEMENT ACTIVE" {
                        break;
                    }
                }
                let b = c.bvr.as_ref().unwrap();
                runs.push(serde_json::json!({"formation":formation,"friendly":b.allies+1,"hostile":b.enemies,"defense":defense,"difficulty":difficulty.label(),"seconds":c.clock,"first_launch_seconds":first_launch,"missiles":c.missiles.len(),"terminal_active_count":active,"enemy_destroyed":c.hits,"player_destroyed":f.crashed,"result":b.result,"all_states_finite":finite}));
                assert!(finite);
            }
        }
    }
    let weapon_trials = weapon_trials();
    let report = serde_json::json!({"model":"BVR tactical and missile diagnostic; kinematic player path, 0.05 s step","runs":runs,"weapon_trials":weapon_trials});
    std::fs::write(
        "tests/bvr-study.json",
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

fn weapon_trials() -> Vec<serde_json::Value> {
    let mut trials = Vec::new();
    for (name, range, retreat, cutoff) in [
        ("70km-head-on", 70000., false, false),
        ("70km-cold", 70000., true, false),
        ("70km-support-lost", 70000., false, true),
        ("70km-support-lost-turn", 70000., false, true),
        ("100km-head-on", 100000., false, false),
    ] {
        let f = flight();
        let mut c = Combat::default();
        start(&mut c, &f, 1, &Menu::default());
        c.targets[0].position = (f.position + f.forward() * range).normalize() * (RADIUS + 11000.);
        c.targets[0].velocity = f.forward() * if retreat { 280. } else { -280. };
        let mut b = c.bvr.take().unwrap();
        launch(&mut c, &mut b, &f, None, Some(0), Weapon::Radar);
        let m = &mut c.missiles[0];
        let mut target = guidance::target(&c.targets[0]);
        let mut miss = f64::INFINITY;
        let mut active = false;
        for _ in 0..21000 {
            if name.ends_with("-turn") && m.age > 12. && m.age < 12.01 {
                target.velocity = f.orientation * DVec3::X * 280.;
            }
            let old_target = target.position;
            let old_missile = m.position;
            target.position += target.velocity * 0.01;
            target.position = target.position.normalize() * (RADIUS + 11000.);
            let up = target.position.normalize();
            target.velocity = (target.velocity - up * target.velocity.dot(up)).normalize() * 280.;
            guidance::fly(
                m,
                target,
                Support {
                    position: f.position,
                    forward: f.forward(),
                    available: !cutoff || m.age < 10.,
                },
                c.sun,
                0.01,
            );
            miss = miss.min(
                damage::closest_relative(old_missile - old_target, m.position - target.position)
                    .0
                    .length(),
            );
            active |= m.gnc.active;
            if miss < 12.
                || m.position.length() - RADIUS < terrain_height(m.position.x, m.position.z)
            {
                break;
            }
        }
        trials.push(serde_json::json!({"name":name,"closest_m":miss,"seconds":m.age,"active":active,"peak_speed_mps":m.gnc.peak_speed,"final_speed_mps":m.velocity.length(),"fuel_kg":m.gnc.propellant,"final_guidance":m.gnc.mode.label()}));
    }
    trials
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn f22_bvr_starts_with_all_six_radar_missiles_on_every_reset() {
        let mut f = flight();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::F22);
        let mut c = Combat::default();
        for _ in 0..2 {
            start(&mut c, &f, 1, &Menu::default());
            assert_eq!(c.ammo, [2, 6]);
            assert_eq!(c.fight.as_ref().unwrap().gun, f.aircraft.spec().gun);
            c.ammo = [0, 0];
        }
    }
    #[test]
    fn bvr_aim9x_profile_is_player_f22_only() {
        let mut f = flight();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::F22);
        let mut c = Combat::default();
        start(&mut c, &f, 1, &Menu::default());
        let mut b = c.bvr.take().unwrap();
        launch(&mut c, &mut b, &f, None, Some(0), Weapon::Heat);
        launch(&mut c, &mut b, &f, Some(0), None, Weapon::Heat);
        launch(&mut c, &mut b, &f, None, Some(0), Weapon::Radar);
        assert!(c.missiles[0].gnc.aim9x);
        assert!(!c.missiles[1].gnc.aim9x);
        assert!(!c.missiles[2].gnc.aim9x);
    }
    #[test]
    fn batched_aircraft_match_serial_physics_with_damage_and_uneven_chunks() {
        let (c, _) = scene(3);
        let mut serial = c.targets;
        serial[0].damage.engine = 0.;
        serial[2].damage.surface_state[1] = damage::SurfaceState::Jammed(0.19);
        serial[4].damage.surface_state[6] = damage::SurfaceState::Detached;
        serial[4].damage.surfaces[6] = 0.;
        serial[7].alive = false;
        let mut parallel = serial.clone();
        let mut environment = Environment::default();
        for _ in 0..120 {
            let dt = 1. / 120.;
            environment.advance(dt);
            let plans: Vec<_> = serial
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    t.alive.then_some(FlightDecision {
                        desired: (t.orientation * -DVec3::Z
                            + t.position.normalize() * 0.1
                            + DVec3::X * (i as f64 * 0.02))
                            .normalize(),
                        speed: 260. + i as f64 * 4.,
                        dir: DVec3::ZERO,
                        memory: None,
                        incoming: false,
                    })
                })
                .collect();
            advance_aircraft(
                &mut serial,
                &plans,
                &environment,
                Difficulty::Hard,
                dt,
                false,
            );
            advance_aircraft(
                &mut parallel,
                &plans,
                &environment,
                Difficulty::Hard,
                dt,
                true,
            );
            for (a, b) in serial.iter().zip(&parallel) {
                assert_eq!(a.position, b.position);
                assert_eq!(a.velocity, b.velocity);
                assert_eq!(a.orientation, b.orientation);
                assert_eq!(a.dynamics.rates, b.dynamics.rates);
                assert_eq!(a.dynamics.surface, b.dynamics.surface);
                assert_eq!(a.dynamics.controls, b.dynamics.controls);
                assert_eq!(a.fuel, b.fuel);
                assert_eq!(a.health, b.health);
                assert_eq!(a.alive, b.alive);
                assert_eq!(a.damage.rng, b.damage.rng);
                assert_eq!(a.brain.physics_steps, b.brain.physics_steps);
                assert_eq!(a.brain.mode, b.brain.mode);
            }
        }
    }
    #[test]
    fn long_range_head_on_trials_reach_fuse_distance_with_active_seekers() {
        for trial in weapon_trials() {
            if trial["name"] == "70km-support-lost-turn" {
                assert!(trial["closest_m"].as_f64().unwrap() > 1000., "{trial}");
                continue;
            }
            if !trial["name"].as_str().unwrap().ends_with("head-on") {
                continue;
            }
            assert!(trial["closest_m"].as_f64().unwrap() < 12., "{trial}");
            assert_eq!(trial["active"], true);
            assert!(trial["final_speed_mps"].as_f64().unwrap() > 500.);
        }
    }
    #[test]
    fn full_player_dynamics_and_bvr_clocks_advance_together() {
        let (mut c, mut f) = scene(3);
        let mut e = Environment::default();
        e.wind_enabled = false;
        for _ in 0..1200 {
            e.advance(1. / 120.);
            simulate(&mut f, &e, 1. / 120.);
            c.tick(&mut f, 1. / 120.);
        }
        assert!(!f.crashed && f.position.is_finite() && f.velocity.is_finite());
        assert!(f.altitude() > 9000.);
        assert!((f.elapsed - c.clock).abs() < 1e-7);
        assert!((c.clock - 10.).abs() < 1e-7);
    }
    #[test]
    fn mountainous_terrain_masks_low_radar_lines_of_sight() {
        let mut masked = 0;
        for x in (60000..150000).step_by(10000) {
            let z = -50000.;
            let x = x as f64;
            let observer = surface_point(x, z, terrain_height(x, z) + 120.);
            let position = surface_point(x + 30000., z, terrain_height(x + 30000., z) + 120.);
            let contact = Contact {
                position,
                velocity: DVec3::Z * 250.,
                forward: DVec3::Z,
                throttle: 0.9,
                engine: true,
                alive: true,
                decoy: false,
                strength: 1.,
            };
            if !guidance::terrain_visible(observer, position) {
                masked += 1;
                assert_eq!(radar_signal(observer, contact), 0.);
            }
        }
        assert!(masked > 0);
    }
    fn scene(formation: usize) -> (Combat, Flight) {
        let f = flight();
        let mut c = Combat::default();
        start(
            &mut c,
            &f,
            1,
            &Menu {
                formation,
                ..default()
            },
        );
        (c, f)
    }
    #[test]
    fn all_force_sizes_have_separate_difficulties_and_125_km_start() {
        for formation in 0..4 {
            for defense in [false, true] {
                for difficulty in [Difficulty::Training, Difficulty::Balanced, Difficulty::Hard] {
                    let f = flight();
                    let mut c = Combat::default();
                    start(
                        &mut c,
                        &f,
                        1,
                        &Menu {
                            formation,
                            defense,
                            difficulty,
                            ..default()
                        },
                    );
                    let b = c.bvr.as_ref().unwrap();
                    assert_eq!((b.enemies, b.allies), FORMATIONS[formation]);
                    assert_eq!(b.difficulty, difficulty);
                    assert_eq!(b.defense, defense);
                    assert_eq!(b.range_at_start, 125000.);
                    assert_eq!(c.targets.len(), b.enemies + b.allies);
                    assert!(c.targets[..b.enemies].iter().all(|t|(100000. ..150000.).contains(&t.position.distance(f.position))));
                }
            }
        }
    }
    #[test]
    fn search_cannot_launch_and_friendlies_are_inhibited() {
        let (mut c, f) = scene(1);
        c.bvr.as_mut().unwrap().radar.mode = RadarMode::Search;
        assert_eq!(c.cue(&f), Err("SELECT TWS OR STT"));
        c.selected = 2;
        assert_eq!(c.cue(&f), Err("FRIENDLY / INHIBIT"));
    }
    #[test]
    fn scans_acquire_tracks_and_power_loss_ages_them_out() {
        let (mut c, mut f) = scene(0);
        for _ in 0..60 {
            c.tick(&mut f, 0.2);
        }
        assert!(
            c.bvr.as_ref().unwrap().radar.tracks[..2]
                .iter()
                .any(|t| t.valid(c.clock))
        );
        f.battery = false;
        for _ in 0..120 {
            c.tick(&mut f, 0.2);
        }
        assert!(
            !c.bvr.as_ref().unwrap().radar.tracks[..2]
                .iter()
                .any(|t| t.valid(c.clock))
        );
    }
    #[test]
    fn scan_range_and_elevation_gate_local_detections() {
        let (mut c, mut f) = scene(0);
        c.bvr.as_mut().unwrap().radar.range = 0;
        for _ in 0..30 {
            c.tick(&mut f, 0.2);
        }
        assert!(
            c.bvr
                .as_ref()
                .unwrap()
                .radar
                .tracks
                .iter()
                .all(|t| !t.valid(c.clock))
        );
        let b = c.bvr.as_mut().unwrap();
        b.radar.range = 3;
        b.radar.elevation = 40.;
        for _ in 0..30 {
            c.tick(&mut f, 0.2);
        }
        assert!(
            c.bvr
                .as_ref()
                .unwrap()
                .radar
                .tracks
                .iter()
                .all(|t| !t.valid(c.clock))
        );
    }
    #[test]
    fn launch_energy_cues_depend_on_altitude_speed_and_closure() {
        let f = flight();
        let t = guidance::target(&scene(0).0.targets[0]);
        let high = envelope(f.position, f.velocity, t);
        let mut away = t;
        away.velocity = -t.velocity;
        let cold = envelope(surface_point(100000., 0., 2000.), f.velocity * 0.5, away);
        assert!(high.0 > cold.0 + 20000.);
        assert!(high.1 < high.0);
    }
    #[test]
    fn mission_results_and_pause_reset_are_consistent() {
        let (mut c, mut f) = scene(3);
        f.paused = true;
        c.tick(&mut f, 10.);
        assert_eq!(c.clock, 0.);
        f.paused = false;
        for t in &mut c.targets[..8] {
            t.alive = false;
        }
        c.tick(&mut f, 0.01);
        assert!(c.bvr.as_ref().unwrap().result.contains("COMPLETE"));
        start(&mut c, &f, 2, &Menu::default());
        assert_eq!(c.clock, 0.);
        assert!(c.missiles.is_empty());
        assert!(
            c.bvr
                .as_ref()
                .unwrap()
                .radar
                .tracks
                .iter()
                .all(|t| t.reports == 0)
        );
    }
    #[test]
    fn defense_breach_fails_while_elimination_does_not() {
        for defense in [false, true] {
            let f = flight();
            let mut c = Combat::default();
            start(
                &mut c,
                &f,
                1,
                &Menu {
                    defense,
                    ..default()
                },
            );
            let goal = c.bvr.as_ref().unwrap().objective;
            c.targets[0].position = goal.normalize() * (RADIUS + 5000.);
            let mut f = f;
            c.tick(&mut f, 0.001);
            assert_eq!(c.bvr.as_ref().unwrap().result.contains("BREACHED"), defense);
        }
    }
    #[test]
    fn bvr_player_missile_hit_reports_destroyed_enemy_once() {
        let (mut c, mut f) = scene(1);
        c.targets[0].damage.rng = 3;
        let t = &c.targets[0];
        c.missiles.push(Missile {
            gnc: guidance::Gnc::launch(t.velocity, guidance::target(t), 1),
            position: t.position,
            velocity: t.velocity,
            kind: Weapon::Radar,
            target: 0,
            age: 1.,
            alive: true,
            guided: true,
            lost_for: 0.,
            trail: vec![],
            trail_clock: 0.,
            decoy: None,
        });
        c.bvr.as_mut().unwrap().shots.push(Shot {
            launcher: None,
            target: Some(0),
            friendly: true,
        });
        c.tick(&mut f, 0.001);
        assert!(!c.targets[0].alive);
        assert_eq!(c.hits, 1);
        assert!(c.recent_hit.as_ref().unwrap().destroyed);
        c.tick(&mut f, 0.001);
        assert_eq!(c.hits, 1);
    }
    #[test]
    fn bvr_cannon_reports_enemy_damage_and_does_not_boost_friendly_damage() {
        let (mut c, f) = scene(1);
        let enemies = c.bvr.as_ref().unwrap().enemies;
        for index in [0, enemies] {
            c.recent_hit = None;
            let t = &c.targets[index];
            let direction = t.orientation * -DVec3::Z;
            let position = t.position - direction * 20.;
            c.fight.as_mut().unwrap().rounds.push(dogfight::Round {
                position,
                previous: position,
                velocity: t.velocity + direction * 4250.,
                hostile: false,
                age: 0.,
                damage: 0.075,
                alive: true,
            });
            cannon(&mut c, &f, 0.01, enemies);
            assert!(c.targets[index].health < 1.);
            if index == 0 {
                let r = c.recent_hit.as_ref().unwrap();
                assert_eq!(r.index, 0);
                assert_eq!(r.integrity, c.targets[0].health);
            } else {
                assert!(c.recent_hit.is_none());
            }
        }
        let enemy_loss = 1. - c.targets[0].health;
        let friendly_loss = 1. - c.targets[enemies].health;
        assert!((enemy_loss / friendly_loss - 4.).abs() < 1e-10);
    }
    #[test]
    fn long_range_profile_sustains_after_standard_motor_burnout() {
        let (mut c, f) = scene(0);
        let mut b = c.bvr.take().unwrap();
        launch(&mut c, &mut b, &f, None, Some(0), Weapon::Radar);
        let m = &mut c.missiles[0];
        let contact = guidance::target(&c.targets[0]);
        for _ in 0..1000 {
            guidance::fly(
                m,
                contact,
                Support {
                    position: f.position,
                    forward: f.forward(),
                    available: true,
                },
                c.sun,
                0.01,
            );
        }
        assert!(m.gnc.thrust > 0.);
        assert!(m.gnc.mass < 190. && m.gnc.mass > 120.);
        assert!(m.velocity.is_finite());
    }
}
