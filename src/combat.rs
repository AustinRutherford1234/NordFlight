//! Target-practice gameplay. Guidance and propulsion are tunable game approximations.
//! All moving objects use f64 world positions and the flight simulation's fixed substeps.
use crate::{FlightCamera, ViewState, physics::*, world};
use bevy::{
    asset::RenderAssetUsages,
    camera::primitives::Aabb,
    math::{DMat3, DQuat, DVec3},
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};
pub mod ai;
pub mod aim9x;
pub mod radar_visual;
pub mod smoke_gpu;
pub mod bvr;
pub mod contact;
pub mod damage;
pub mod dogfight;
pub mod effects;
pub mod guidance;
pub mod hud;
pub mod rwr;

const TARGET_COUNT: usize = 3;
const MAX_AIRCRAFT: usize = 11;
// Mechanism children never carry the generic presentation roles. These filters
// let Bevy prove independent transform/visibility access without relaxing any
// input, simulation, camera, or damage ordering.
pub type MechanismFilter = (Without<TargetVisual>,Without<MissileVisual>,Without<MotorVisual>,Without<FlashVisual>,Without<StoreVisual>,Without<GenericMissileDecoration>);
/// Gameplay muzzle velocity relative to the firing aircraft (m/s).
pub(crate) const CANNON_MUZZLE_SPEED: f64 = 4250.0;
// Reserve stable player slots for the largest aircraft loadout. Enemy slots
// must not overlap later player launches or change as missiles are fired.
pub(super) const MISSILE_COUNT: usize = 10;
const RENDER_MISSILES: usize = 64;
const STATIONS: [DVec3; 4] = [
    DVec3::new(-3.15, -0.48, 0.9),
    DVec3::new(3.15, -0.48, 0.9),
    DVec3::new(-2.05, -0.65, 0.1),
    DVec3::new(2.05, -0.65, 0.1),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Weapon {
    Heat,
    Radar,
}
impl Weapon {
    fn player_range(self, f: &Flight) -> f64 {
        if self == Self::Heat && matches!(f.aircraft,crate::aircraft::Aircraft::F22|crate::aircraft::Aircraft::Hornet) { 10000.0 } else { self.range() }
    }
    fn player_cone(self, f: &Flight) -> f64 {
        if self == Self::Heat {
            if matches!(f.aircraft,crate::aircraft::Aircraft::F22|crate::aircraft::Aircraft::Hornet) { 60.0 } else { 16.0 }
        } else { 35.0 }
    }
    pub(crate) fn index(self) -> usize {
        if self == Self::Heat { 0 } else { 1 }
    }
    fn label(self) -> &'static str {
        if self == Self::Heat {
            "HEAT SEEKER"
        } else {
            "ACTIVE RADAR"
        }
    }
    fn range(self) -> f64 {
        if self == Self::Heat { 6000.0 } else { 20000.0 }
    }
    pub(crate) fn lock_time(self) -> f64 {
        if self == Self::Heat { 0.8 } else { 1.3 }
    }
    fn fuse(self) -> f64 {
        if self == Self::Heat { 12.0 } else { 18.0 }
    }
    fn burn(self) -> f64 {
        if self == Self::Heat { 4.5 } else { 6.0 }
    }
}

#[derive(Clone)]
pub struct Target {
    pub dynamics: Flight,
    pub brain: ai::Brain,
    pub damage: AircraftDamage,
    pub wreck_age: f64,
    pub throttle: f64,
    pub fuel: f64,
    pub health: f64,
    pilot: Option<dogfight::Pilot>,
    pub position: DVec3,
    pub velocity: DVec3,
    pub orientation: DQuat,
    pub alive: bool,
    center: DVec3,
    right: DVec3,
    forward: DVec3,
    phase: f64,
    altitude: f64,
    speed: f64,
}
impl Target {
    fn advance(&mut self, dt: f64, e: &Environment) {
        self.phase += self.speed * dt / 10000.;
        let angle = self.phase + 0.12;
        let goal = self.center + (-self.right * angle.cos() + self.forward * angle.sin()) * 10000.;
        let goal = goal.normalize() * (RADIUS + self.altitude);
        self.brain.mode = "ROUTE";
        ai::fly(
            self,
            e,
            (goal - self.position).normalize_or(self.forward),
            self.speed,
            dogfight::Difficulty::Hard,
            dt,
        );
    }
}

#[derive(Clone)]
pub struct Missile {
    pub gnc: guidance::Gnc,
    decoy: Option<u64>,
    pub position: DVec3,
    pub velocity: DVec3,
    pub kind: Weapon,
    pub target: usize,
    pub age: f64,
    pub alive: bool,
    pub guided: bool,
    lost_for: f64,
    trail: Vec<DVec3>,
    trail_clock: f64,
}
#[derive(Clone)]
struct Impact {
    position: DVec3,
    age: f64,
    hit: bool,
}
#[derive(Resource)]
pub struct Combat {
    pub environment: Environment,
    pub bvr: Option<bvr::Battle>,
    pub radar: bvr::Radar,
    pub(crate) audio_events: std::collections::VecDeque<crate::sound::Event>,
    pub sun: DVec3,
    pub(crate) fight: Option<dogfight::Fight>,
    pub active: bool,
    pub fx: effects::State,
    pub recent_hit: Option<effects::HitReport>,
    generation: u64,
    pub targets: Vec<Target>,
    pub missiles: Vec<Missile>,
    pub selected: usize,
    pub weapon: Weapon,
    pub ammo: [u8; 2],
    pub lock: f64,
    pub hits: usize,
    pub strikes: usize,
    pub fired: usize,
    clock: f64,
    contact_pairs: [bool; 66],
    old_poses: Vec<(DVec3, DQuat)>,
    last_launch: f64,
    pending_side: Option<PendingSideLaunch>,
    pending_main: Option<PendingMainLaunch>,
    impacts: Vec<Impact>,
    message: String,
    message_until: f64,
}
impl Default for Combat {
    fn default() -> Self {
        Self {
            environment: Environment::default(),
            bvr: None,
            radar: bvr::Radar::default(),
            audio_events: std::collections::VecDeque::with_capacity(64),
            sun: Environment::default().sun_direction(),
            fight: None,
            active: false,
            fx: effects::State::default(),
            recent_hit: None,
            generation: 0,
            targets: vec![],
            missiles: vec![],
            selected: 0,
            weapon: Weapon::Heat,
            ammo: [2, 2],
            lock: 0.0,
            hits: 0,
            strikes: 0,
            fired: 0,
            clock: 0.0,
            contact_pairs: [false; 66],
            old_poses: Vec::with_capacity(MAX_AIRCRAFT),
            last_launch: -10.0,
            pending_side: None,
            pending_main: None,
            impacts: vec![],
            message: String::new(),
            message_until: 0.0,
        }
    }
}
#[derive(Clone, Copy)]
struct PendingSideLaunch { side:usize, target:usize, generation:u64, requested_at:f64 }
#[derive(Clone, Copy)]
struct PendingMainLaunch { slot:usize, target:usize, generation:u64, requested_at:f64 }
pub fn flight() -> Flight {
    let mut f = Flight::approach();
    f.position = surface_point(0.0, -1500.0, 2000.0);
    f.velocity = DVec3::new(0.0, 0.0, -230.0);
    f.orientation = DQuat::from_rotation_x(0.025);
    f.throttle = 0.70;
    f.spool = 0.70;
    f.fuel = 2400.0;
    f.gear = false;
    f.gear_extension = 0.0;
    f.gear_door = 0.0;
    f.flaps = 0.0;
    let (temperature, pressure, density, sound) = atmosphere(f.altitude());
    f.temperature = temperature;
    f.pressure = pressure;
    f.density = density;
    f.airspeed = f.velocity.length();
    f.mach = f.airspeed / sound;
    f.thrust = crate::propulsion::jet_output(f.spool, density, f.mach).0;
    f.status = "TARGET PRACTICE  /  TAB SELECT · O WEAPON · SPACE FIRE".into();
    f
}
fn facing(forward: DVec3, up: DVec3) -> DQuat {
    let right = forward.cross(up).normalize();
    DQuat::from_mat3(&DMat3::from_cols(
        right,
        right.cross(forward).normalize(),
        -forward,
    ))
}
impl Combat {
    /// Input emulation for the opt-in validation driver; uses gameplay paths.
    pub(crate) fn diagnostic_controls(&mut self, f: &Flight, gun: bool, launch: bool, decoys: bool) {
        if let Some(fight)=&mut self.fight { fight.gun_held=gun; }
        if launch { self.fire(f); }
        if decoys { self.deploy_pair(f); }
    }
    pub(crate) fn diagnostic_damage(&mut self) {
        if let Some(target)=self.targets.iter_mut().find(|t|t.alive) {
            // Deterministic posed damage exercises visual pools; this is not a
            // claim of a weapon hit or a substitute for hit-model regressions.
            target.damage.fire=0.7; target.damage.fuel_leak=0.5;
        }
    }
    pub(crate) fn sound_impacts(&self, dt: f64) -> impl Iterator<Item = DVec3> + '_ {
        self.impacts
            .iter()
            .filter(move |p| p.age < dt * (1.0 - 1e-7))
            .map(|p| p.position)
    }
    pub fn prepare_capture(&mut self, f: &mut Flight, environment:&Environment, mode: &str) {
        for _ in 0..180 {
            self.advance(f, 1.0 / 120.0);
        }
        if matches!(
            mode,
            "combat-launch" | "combat-flight" | "combat-follow" | "combat-impact" | "combat-vector"
        ) {
            if std::env::var("NORD_MISSILE_KIND").as_deref()==Ok("radar") {
                self.weapon=Weapon::Radar;
                self.lock=Weapon::Radar.lock_time();
            }
            if mode=="combat-vector" {
                // Disposable visual check: a real off-axis launch demand, not a
                // decorative oscillation of surfaces independent of the GNC.
                let target=&mut self.targets[0];
                target.position+=f.orientation*DVec3::new(600.,150.,0.);
                target.dynamics.position=target.position;
            }
            self.fire(f);
            // Exercise the same sequencer as live flight; a closed side bay
            // cannot be bypassed just because this is a native visual capture.
            for _ in 0..240 {
                if self.pending_side.is_none()&&self.pending_main.is_none(){break;}
                crate::physics::simulate(f,environment,1./120.);self.tick(f,1./120.);
            }
            let steps = match mode {
                "combat-launch" | "combat-vector" => 48,
                "combat-flight" | "combat-follow" => 150,
                _ => 3600,
            };
            for _ in 0..steps {
                self.advance(f, 1.0 / 120.0);
                if mode == "combat-impact" && self.hits > 0 {
                    break;
                }
            }
            if let Some(m)=self.missiles.first().filter(|m|m.gnc.aim9x) {
                println!("AIM-9X native pose: age={:.3}, thrust={:.1}, controller={:?}, body={:?}",m.age,m.gnc.thrust,m.gnc.fins,m.gnc.body);
            }
            if mode == "combat-impact" && self.hits > 0 {
                for _ in 0..42 {
                    self.advance(f, 1.0 / 120.0);
                }
            }
        }
    }
    pub fn start(&mut self, f: &Flight, generation: u64) {
        *self = Self {
            active: true,
            generation,
            ammo: f.aircraft.spec().ammo,
            ..default()
        };
        let up = f.up();
        let forward = (f.forward() - up * f.forward().dot(up)).normalize();
        let right = forward.cross(up).normalize();
        for (x, distance, h, speed) in [
            (-90.0, 1200.0, 0.0, 170.0),
            (220.0, 2500.0, 160.0, 185.0),
            (-360.0, 4300.0, -100.0, 195.0),
        ] {
            let origin = f.position + right * x + forward * distance + up * h;
            let mut target = Target {
                dynamics: ai::airframe(),
                brain: ai::Brain {
                    id: self.targets.len(),
                    ..default()
                },
                damage: AircraftDamage {
                    rng: generation
                        .wrapping_mul(0x9e3779b97f4a7c15)
                        .wrapping_add(self.targets.len() as u64 * 131 + 3),
                    ..default()
                },
                wreck_age: 0.0,
                throttle: 0.72,
                fuel: 2400.0,
                health: 1.0,
                pilot: None,
                position: origin,
                velocity: forward * speed,
                orientation: facing(forward, up),
                alive: true,
                center: origin + right * 10000.0,
                right,
                forward,
                phase: 0.0,
                altitude: origin.length() - RADIUS,
                speed,
            };
            target.advance(0.0, &self.environment);
            ai::initialize(&mut target);
            self.targets.push(target);
        }
    }
    pub(crate) fn cue(&self, f: &Flight) -> Result<(), &'static str> {
        if f.aircraft==crate::aircraft::Aircraft::Hornet&&f.systems.hornet_wing_fold>0.02 {return Err("UNFOLD WINGS");}
        if let Some(b) = &self.bvr {
            return b.cue(self, f);
        }
        if self.weapon == Weapon::Radar && !self.radar.powered {
            return Err("RADAR STANDBY");
        }
        if self.weapon == Weapon::Radar && f.damage.radar < 0.25 {
            return Err("RADAR FAILED / USE HEAT SEEKER");
        }
        if !f.battery {
            return Err("POWER OFF");
        }
        let Some(target) = self.targets.get(self.selected).filter(|t| t.alive) else {
            return Err("NO TARGET");
        };
        let r = target.position - f.position;
        let range = r.length();
        if range < 250.0 {
            return Err("TOO CLOSE");
        }
        if range > self.weapon.player_range(f) {
            return Err("OUT OF RANGE");
        }
        let cone = self.weapon.player_cone(f);
        if f.forward().dot(r.normalize()) < cone.to_radians().cos() {
            return Err("POINT NOSE AT TARGET");
        }
        let (signal, reason) =
            guidance::quality(self.weapon, f.position, guidance::target(target), self.sun);
        if signal < 0.18 {
            return Err(reason);
        }
        if f.crashed {
            return Err("AIRFRAME LOST");
        }
        Ok(())
    }
    fn notify(&mut self, text: impl Into<String>) {
        self.message = text.into();
        self.message_until = self.clock + 3.0;
    }
    pub(crate) fn cycle(&mut self) {
        for offset in 1..=self.targets.len() {
            let index = (self.selected + offset) % self.targets.len();
            if self.targets[index].alive
                && self
                    .bvr
                    .as_ref()
                    .is_none_or(|b| index < b.enemies && b.radar.tracks[index].valid(self.clock))
            {
                self.selected = index;
                break;
            }
        }
        self.lock = 0.0;
    }
    fn internal_main_slot(&self,f:&Flight)->Option<usize> {
        if f.aircraft!=crate::aircraft::Aircraft::F22 || self.weapon!=Weapon::Radar{return None;}
        let slot=f.aircraft.spec().ammo[1].saturating_sub(self.ammo[1]) as usize;
        (slot<6&&!(f.systems.external_stores&&slot>=4)).then_some(slot)
    }
    fn queue_main_launch(&mut self,f:&Flight)->Option<bool> {
        let slot=self.internal_main_slot(f)?;
        if self.pending_main.is_some(){return Some(true);}
        if !f.systems.stores[slot+2]{self.notify("MAIN BAY ROUND UNAVAILABLE");return Some(false);}
        if f.gear_extension>0.05{self.notify("RETRACT LANDING GEAR");return Some(false);}
        if !crate::aircraft::main_bay::ready(&f.systems,slot) {
            self.pending_main=Some(PendingMainLaunch{slot,target:self.selected,generation:self.generation,requested_at:self.clock});
            self.notify("MAIN BAY OPENING / LAUNCH QUEUED");return Some(true);
        }
        None
    }
    fn service_main_launch(&mut self,f:&mut Flight) {
        let Some(p)=self.pending_main else{f.systems.main_request=None;return;};
        let invalid=!self.active||f.crashed||f.vehicle!=Vehicle::Gripen
            ||self.internal_main_slot(f)!=Some(p.slot)||self.ammo[1]==0
            ||self.generation!=p.generation||self.selected!=p.target
            ||self.targets.get(p.target).is_none_or(|t|!t.alive)
            ||!f.systems.stores[p.slot+2]||f.damage.hydraulics<=0.12
            ||f.gear_extension>0.05||self.clock-p.requested_at>3.||self.cue(f).is_err();
        if invalid {
            self.pending_main=None;f.systems.main_request=None;
            self.notify("MAIN BAY LAUNCH CANCELLED");return;
        }
        f.systems.main_request=Some(p.slot);
        if crate::aircraft::main_bay::ready(&f.systems,p.slot)&&self.lock>=self.weapon.lock_time() {
            self.pending_main=None;
            if self.fire(f){f.systems.stores[p.slot+2]=false;}
            f.systems.main_request=None;
        }
    }
    fn queue_side_launch(&mut self,f:&Flight)->Option<bool> {
        if f.aircraft!=crate::aircraft::Aircraft::F22 || self.weapon!=Weapon::Heat {return None;}
        if self.pending_side.is_some() {return Some(true);}
        let side=f.aircraft.spec().ammo[0].saturating_sub(self.ammo[0]) as usize;
        if side>=2 || !f.systems.stores[side] {self.notify("SIDE BAY ROUND UNAVAILABLE");return Some(false);}
        if !f.systems.side_bays[side].ready() {
            self.pending_side=Some(PendingSideLaunch {side,target:self.selected,generation:self.generation,requested_at:self.clock});
            self.notify("SIDE BAY OPENING / LAUNCH QUEUED");return Some(true);
        }
        None
    }
    fn service_side_launch(&mut self,f:&mut Flight) {
        let Some(p)=self.pending_side else {f.systems.side_request=None;return;};
        let invalid=!self.active || f.crashed || f.vehicle!=Vehicle::Gripen
            || f.aircraft!=crate::aircraft::Aircraft::F22 || self.weapon!=Weapon::Heat
            || self.generation!=p.generation || self.selected!=p.target
            || self.targets.get(p.target).is_none_or(|t|!t.alive)
            || self.ammo[0]==0 || self.clock-p.requested_at>3.
            || f.aircraft.spec().ammo[0].saturating_sub(self.ammo[0]) as usize!=p.side
            || !f.systems.stores[p.side]
            || f.damage.hydraulics<=0.12 || f.gear_extension>0.05
            || self.cue(f).is_err();
        if invalid {
            self.pending_side=None;f.systems.side_request=None;
            self.notify("SIDE BAY LAUNCH CANCELLED");return;
        }
        f.systems.side_request=Some(p.side);
        if f.systems.side_bays[p.side].ready() && self.lock>=self.weapon.lock_time() {
            self.pending_side=None;
            // fire() sees the same ready pose used by the attached missile.
            if self.fire(f) {f.systems.stores[p.side]=false;}
            f.systems.side_request=None;
        }
    }
    pub fn fire(&mut self, f: &Flight) -> bool {
        if self.bvr.is_some() {
            return bvr::fire(self, f);
        }
        if !self.active {
            return false;
        }
        if f.paused {
            self.notify("RESUME TO LAUNCH");
            return false;
        }
        if f.crashed {self.notify("AIRFRAME LOST");return false;}
        if f.gear_extension > 0.05 {
            self.notify("RETRACT LANDING GEAR");
            return false;
        }
        if let Err(reason) = self.cue(f) {
            self.notify(reason);
            return false;
        }
        if self.lock < self.weapon.lock_time() {
            self.notify("WAIT FOR SEEKER LOCK");
            return false;
        }
        if self.ammo[self.weapon.index()] == 0 {
            self.notify("NO MISSILES OF THIS TYPE");
            return false;
        }
        if self.clock - self.last_launch < 0.6 {
            return false;
        }
        if let Some(accepted)=self.queue_main_launch(f) {return accepted;}
        if let Some(accepted)=self.queue_side_launch(f) {return accepted;}
        let station=if f.aircraft==crate::aircraft::Aircraft::Gripen {let slot=self.weapon.index()*2+(2-self.ammo[self.weapon.index()])as usize;STATIONS[slot]}else{crate::aircraft::launch_station(f,self.weapon,self.ammo)};
        let position = f.position + f.orientation * station;
        let release_rotation=if f.aircraft==crate::aircraft::Aircraft::F22 && self.weapon==Weapon::Heat {
            let i=f.aircraft.spec().ammo[0].saturating_sub(self.ammo[0]) as usize;
            crate::aircraft::bays::missile_pose(i.min(1),f.systems.side_bays[i.min(1)].launcher).rotation.as_dquat()
        }else{DQuat::IDENTITY};
        let main=self.internal_main_slot(f).is_some();
        let inherited=f.velocity+(f.orientation*f.rates).cross(f.orientation*station);
        let launch_velocity=if main{inherited-f.orientation*DVec3::Y*8.}else{inherited+f.orientation*release_rotation*(-DVec3::Z*18.)};
        let mut gnc=guidance::Gnc::launch(launch_velocity,guidance::target(&self.targets[self.selected]),
            self.generation.wrapping_mul(997)+self.fired as u64+1).for_aircraft(f.aircraft,self.weapon);
        gnc=gnc.inherit(f.orientation*release_rotation,f.orientation*f.rates);
        self.missiles.push(Missile {
            gnc,
            decoy: None,
            position,
            velocity: launch_velocity - (f.orientation * DVec3::Y) * if main||(self.weapon==Weapon::Heat && matches!(f.aircraft,crate::aircraft::Aircraft::F22|crate::aircraft::Aircraft::Hornet)) {0.}else{4.},
            kind: self.weapon,
            target: self.selected,
            age: 0.0,
            alive: true,
            guided: true,
            lost_for: 0.0,
            trail: vec![position],
            trail_clock: 0.0,
        });
        crate::sound::emit(
            &mut self.audio_events,
            crate::sound::Kind::Launch,
            position,
            f.velocity,
            true,
        );
        self.ammo[self.weapon.index()] -= 1;
        self.fired += 1;
        self.last_launch = self.clock;
        self.notify(format!(
            "{} AWAY / TARGET {:02}",
            crate::aircraft::missile_name(f.aircraft, self.weapon),
            self.selected + 1
        ));
        true
    }
    pub fn advance(&mut self, f: &Flight, dt: f64) {
        if !self.active || f.vehicle != Vehicle::Gripen {
            if f.vehicle == Vehicle::Gripen {
                self.clock = f.elapsed;
                bvr::sense_local(self, f);
            }
            return;
        }
        self.clock += dt;
        // Formation capacity is bounded; snapshots never allocate per substep.
        let mut alive_before = [false; MAX_AIRCRAFT];
        let mut old_targets = [DVec3::ZERO; MAX_AIRCRAFT];
        for (i,t) in self.targets.iter().enumerate() {
            alive_before[i]=t.alive;
            old_targets[i]=t.position;
        }
        if let Some(fight) = &self.fight {
            dogfight::advance_targets(
                &mut self.targets,
                &self.missiles,
                f,
                fight.difficulty,
                self.sun,
                self.radar.powered,
                &self.environment,
                dt,
            );
        } else {
            for target in &mut self.targets {
                target.advance(dt, &self.environment);
            }
        }
        for (was_alive, target) in alive_before.into_iter().zip(&self.targets) {
            if was_alive && !target.alive {
                self.hits += 1;
                self.impacts.push(Impact {
                    position: target.position,
                    age: 0.0,
                    hit: true,
                });
            }
        }
        bvr::sense_local(self, f);
        if self.cue(f).is_ok() {
            let signal = guidance::quality(
                self.weapon,
                f.position,
                guidance::target(&self.targets[self.selected]),
                self.sun,
            )
            .0;
            self.lock = (self.lock + dt * signal).min(self.weapon.lock_time());
        } else {
            self.lock = 0.0;
        }
        for impact in &mut self.impacts {
            impact.age += dt;
        }
        self.impacts.retain(|i| i.age < 4.0);
        for missile in &mut self.missiles {
            if !missile.alive {
                continue;
            }
            let old = missile.position;
            let target = &self.targets[missile.target];
            let contact = guidance::target(&target);
            if missile.gnc.gimbal.length_squared() < 0.5 {
                missile.gnc.gimbal = missile.velocity.normalize_or_zero();
            }
            if let Some(fight) = &mut self.fight {
                if missile.decoy.is_none() && (missile.kind == Weapon::Heat || missile.gnc.active) {
                    let kind = missile.kind;
                    let target_id = missile.target;
                    for d in fight
                        .decoys
                        .iter()
                        .filter(|d| d.owner == Some(target_id) && d.kind == kind)
                    {
                        if guidance::consider_decoy(
                            missile,
                            contact,
                            dogfight::decoy_contact(d),
                            d.age,
                            d.id,
                            self.sun,
                        ) {
                            missile.decoy = Some(d.id);
                            break;
                        }
                    }
                }
                if missile.decoy.is_some_and(|id| {
                    !fight.decoys.iter().any(|d| {
                        d.id == id
                            && guidance::retain_decoy(
                                missile,
                                contact,
                                dogfight::decoy_contact(d),
                                d.age,
                                self.sun,
                            )
                    })
                }) {
                    missile.decoy = None;
                }
            }
            let tracked = missile
                .decoy
                .and_then(|id| self.fight.as_ref()?.decoys.iter().find(|d| d.id == id))
                .map_or(contact, dogfight::decoy_contact);
            let support = guidance::Support {
                position: f.position,
                forward: f.forward(),
                available: !f.crashed
                    && f.battery
                    && f.damage.radar >= 0.25
                    && self.selected == missile.target,
            };
            missile.gnc.wind = self.environment.wind(missile.position);
            guidance::fly(missile, tracked, support, self.sun, dt);
            if let Some(hit)=crate::carrier::collider::sweep(f,old,missile.position,dt,0.12) {
                missile.position=hit.point;missile.alive=false;
                self.impacts.push(Impact{position:hit.point,age:0.,hit:false});
                continue;
            }
            let (miss, _) = damage::closest_relative(
                old - old_targets[missile.target],
                missile.position - target.position,
            );
            let body = target.orientation.conjugate() * miss;
            let hit = target.alive
                && missile.age > 0.4
                && miss.length() < missile.kind.fuse() + 9.0
                && crate::airframe::distance_to_skin(body) < missile.kind.fuse();
            if hit {
                missile.alive = false;
                self.strikes += 1;
                let target = &mut self.targets[missile.target];
                let body = target.orientation.conjugate() * miss;
                let event = damage::enemy_missile(
                    &mut target.damage,
                    &mut target.health,
                    body,
                    missile.kind,
                    crate::control_surfaces::angles(
                        target.dynamics.surface,
                        target.dynamics.flaps,
                        target.dynamics.airbrake,
                    ),
                );
                let destroyed = damage::fatal(&target.damage, target.health);
                if destroyed {
                    target.alive = false;
                    target.wreck_age = 0.0;
                    self.hits += 1;
                }
                self.impacts.push(Impact {
                    position: target.position + miss,
                    age: 0.0,
                    hit: true,
                });
                self.message = format!(
                    "TARGET {:02} {} / HULL {:.0}% · ENG {:.0}% / {}",
                    missile.target + 1,
                    if destroyed { "DESTROYED" } else { "DAMAGED" },
                    target.health * 100.0,
                    target.damage.engine * 100.0,
                    event.label()
                );
                self.message_until = self.clock + 4.0;
                self.recent_hit = Some(effects::HitReport::new(missile.target, target, self.clock));
            } else if missile.position.length() - RADIUS
                < terrain_height(missile.position.x, missile.position.z).max(0.0)
                || missile.age > 45.0
                || !missile.position.is_finite()
            {
                missile.alive = false;
                self.impacts.push(Impact {
                    position: missile.position,
                    age: 0.0,
                    hit: false,
                });
            }
        }
        if self.targets.get(self.selected).is_some_and(|t| !t.alive) {
            self.cycle();
        }
    }
}
fn swept_distance(start: DVec3, end: DVec3) -> f64 {
    let delta = end - start;
    let t = (-start.dot(delta) / delta.length_squared().max(1e-12)).clamp(0.0, 1.0);
    (start + delta * t).length()
}

#[derive(Component)]
pub struct CombatButton(u8);
#[derive(Component)]
pub struct CombatPanel;
#[derive(Component)]
pub struct CombatReadout;
#[derive(Component)]
pub struct MissileReadout;
#[derive(Component)]
pub struct TargetMarker(usize);
#[derive(Component)]
pub struct TargetLabel(usize);
#[derive(Component)]
pub(crate) struct CockpitIconsButton;
#[derive(Component)]
pub(crate) struct CockpitIconsLabel;

pub fn cockpit_icon_controls(
    keys: Res<ButtonInput<KeyCode>>,
    f: Res<Flight>,
    mut v: ResMut<ViewState>,
    mut buttons: Query<(Ref<Interaction>, &mut Node, &mut BackgroundColor), With<CockpitIconsButton>>,
    mut labels: Query<&mut Text, With<CockpitIconsLabel>>,
) {
    let cockpit = v.index == 3 && !v.map && !v.following() && f.vehicle == Vehicle::Gripen;
    let clicked = buttons.iter().any(|(interaction, _, _)| *interaction == Interaction::Pressed
        && interaction.is_changed());
    if cockpit && ((keys.just_pressed(KeyCode::KeyH)
        && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight)))
        || (clicked && !v.left_camera_drag)) {
        v.cockpit_enemy_icons = !v.cockpit_enemy_icons;
    }
    for (_, mut node, mut color) in &mut buttons {
        let display = if cockpit { Display::Flex } else { Display::None };
        if node.display != display { node.display = display; }
        let tint = if v.cockpit_enemy_icons { Color::srgb(0.08, 0.25, 0.24) }
            else { Color::srgba(0.025, 0.065, 0.075, 0.9) };
        if color.0 != tint { color.0 = tint; }
    }
    for mut text in &mut labels {
        let label = if v.cockpit_enemy_icons { "ALT+H / ENEMY ICONS ON" } else { "ALT+H / ENEMY ICONS OFF" };
        if text.0 != label { text.0 = label.into(); }
    }
}

fn target_marker_visible(combat: &Combat, f: &Flight, v: &ViewState, index: usize) -> bool {
    if !combat.active || f.vehicle != Vehicle::Gripen || v.map || v.following()
        || !combat.targets.get(index).is_some_and(|t| t.alive) {
        return false;
    }
    if v.index == 3 {
        v.cockpit_enemy_icons && !f.crashed
            && !combat.bvr.as_ref().is_some_and(|b| b.friendly(index))
    } else {
        combat.bvr.is_none() && !v.simple && index != combat.selected
    }
}
#[derive(Component)]
pub struct TargetVisual(usize);
#[derive(Component)]
pub struct TargetControl {
    pub target: usize,
    pub index: usize,
    pub axis: Vec3,
}
#[derive(Component)]
pub struct MissileVisual(usize);
#[derive(Component)]
pub struct MotorVisual(usize, usize);
#[derive(Component)]
pub struct SmokeVisual(usize);
#[derive(Component)]
pub struct StoreVisual(usize, bool);
#[derive(Component)]
pub struct FlashVisual(usize);

fn mesh(vertices: Vec<Vec3>, indices: Vec<u32>) -> Mesh {
    let n = vertices.len();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vertices
            .into_iter()
            .map(|p| p.to_array())
            .collect::<Vec<_>>(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n])
    .with_inserted_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();
    mesh
}
fn body_mesh() -> Mesh {
    let mut p = vec![];
    let mut idx = vec![];
    let sections = [
        (-7.0, 0.03, 0.03),
        (-5.0, 0.42, 0.34),
        (-3.0, 0.65, 0.57),
        (0.0, 0.84, 0.66),
        (3.5, 0.72, 0.59),
        (5.6, 0.51, 0.49),
    ];
    for (z, w, h) in sections {
        for i in 0..32 {
            let a = i as f32 * std::f32::consts::TAU / 32.0;
            p.push(Vec3::new(w * a.cos(), h * a.sin(), z));
        }
    }
    for j in 0..sections.len() - 1 {
        for i in 0..32 {
            let a = (j * 32 + i) as u32;
            let b = (j * 32 + (i + 1) % 32) as u32;
            idx.extend([a, b, a + 32, b, b + 32, a + 32]);
        }
    }
    mesh(p, idx)
}
fn surfaces_mesh() -> Mesh {
    let mut p = vec![];
    let mut idx = vec![];
    let mut triangle = |a: Vec3, b: Vec3, c: Vec3| {
        let n = p.len() as u32;
        p.extend([a, b, c]);
        idx.extend([n, n + 1, n + 2]);
    };
    for side in [-1.0, 1.0] {
        triangle(
            Vec3::new(side * 0.65, -0.10, -1.1),
            Vec3::new(side * 4.15, -0.12, 2.9),
            Vec3::new(side * 0.65, -0.10, 3.4),
        );
    }
    triangle(
        Vec3::new(0.0, 0.45, 1.3),
        Vec3::new(0.0, 3.1, 3.7),
        Vec3::new(0.0, 0.4, 4.3),
    );
    mesh(p, idx)
}
fn target_control_mesh(i: usize) -> (Mesh, Vec3, Vec3) {
    let point = damage::SURFACE_POINTS[i].as_vec3();
    let side = if i < 4 { -1.0 } else { 1.0 };
    let (vertices, axis) = match i {
        0 | 4 => (
            vec![
                Vec3::new(side * 0.5, 0.2, -3.5),
                Vec3::new(side * 2.0, 0.17, -2.0),
                Vec3::new(side * 0.5, 0.2, -1.55),
            ],
            Vec3::X,
        ),
        1 | 5 => (
            vec![
                Vec3::new(side * 0.65, -0.1, 3.4),
                Vec3::new(side * 2.4, -0.12, 3.15),
                Vec3::new(side * 2.4, -0.12, 3.9),
                Vec3::new(side * 0.65, -0.1, 4.35),
            ],
            Vec3::X,
        ),
        2 | 6 => (
            vec![
                Vec3::new(side * 2.4, -0.12, 3.15),
                Vec3::new(side * 4.15, -0.12, 2.9),
                Vec3::new(side * 4.15, -0.12, 3.25),
                Vec3::new(side * 2.4, -0.12, 3.9),
            ],
            Vec3::X,
        ),
        3 | 7 => (
            vec![
                Vec3::new(side * 0.7, 0.4, 3.5),
                Vec3::new(side * 0.45, 0.4, 3.5),
                Vec3::new(side * 0.45, 0.4, 4.4),
                Vec3::new(side * 0.7, 0.4, 4.4),
            ],
            Vec3::X,
        ),
        _ => (
            vec![
                Vec3::new(0.0, 3.1, 3.7),
                Vec3::new(0.0, 2.8, 4.3),
                Vec3::new(0.0, 0.4, 5.4),
                Vec3::new(0.0, 0.4, 4.3),
            ],
            Vec3::Y,
        ),
    };
    let indices = if vertices.len() == 3 {
        vec![0, 1, 2]
    } else {
        vec![0, 1, 2, 0, 2, 3]
    };
    (
        mesh(vertices.into_iter().map(|p| p - point).collect(), indices),
        point,
        axis,
    )
}
pub(crate) fn missile_mesh() -> Mesh {
    missile_mesh_profile(false)
}
/// NAVAIR dimensions: 3.02 m length, 5-inch body, 0.45 m overall fin span.
pub(crate) fn aim9x_mesh() -> Mesh {
    aim9x::body_mesh(true)
}
fn missile_mesh_profile(aim9x: bool) -> Mesh {
    let mut p = vec![];
    let mut idx = vec![];
    let rings: &[(f32, f32)] = if aim9x {
        &[(-1.51, 0.010), (-1.30, 0.0635), (-1.29, 0.0635), (1.36, 0.0635), (1.51, 0.050)]
    } else { &[(-1.65, 0.005), (-1.22, 0.075), (1.25, 0.085), (1.38, 0.060)] };
    for &(z, r) in rings {
        for i in 0..24 {
            let a = i as f32 * std::f32::consts::TAU / 24.0;
            p.push(Vec3::new(r * a.cos(), r * a.sin(), z));
        }
    }
    // Close the seeker and motor ends rather than leaving open cylinders.
    for (ring, reverse) in [(0_u32, true), (((rings.len()-1)*24) as u32, false)] {
        for i in 1..23 {
            let triangle = if reverse { [ring, ring+i+1, ring+i] } else { [ring, ring+i, ring+i+1] };
            idx.extend(triangle);
        }
    }
    for j in 0..rings.len()-1 {
        for i in 0..24 {
            let a = (j * 24 + i) as u32;
            let b = (j * 24 + (i + 1) % 24) as u32;
            idx.extend([a, b, a + 24, b, b + 24, a + 24]);
        }
    }
    for angle in [
        0.0,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
        3.0 * std::f32::consts::FRAC_PI_2,
    ] {
        let fins = if aim9x { [(-1.03, -0.68, 0.16), (0.93, 1.46, 0.225)] }
            else { [(-0.45, 0.1, 0.22), (0.72, 1.28, 0.33)] };
        for (front, rear, span) in fins {
            let n = p.len() as u32;
            let radial = Vec3::new(angle.cos(), angle.sin(), 0.0);
            p.extend([
                radial * if aim9x { 0.0635 } else { 0.08 } + Vec3::Z * front,
                radial * span + Vec3::Z * rear,
                radial * if aim9x { 0.0635 } else { 0.08 } + Vec3::Z * rear,
            ]);
            idx.extend([n, n + 1, n + 2]);
        }
    }
    let colors: Vec<[f32; 4]> = (0..p.len()).map(|i| if i<48 {
        [0.025, 0.045, 0.055, 1.0]
    } else { [0.82, 0.84, 0.81, 1.0] }).collect();
    let mut result = mesh(p, idx);
    if aim9x { result.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors); }
    result
}
#[derive(Resource)]
pub struct MissileMeshes {
    generic: Handle<Mesh>, aim9x: Handle<Mesh>,
    motors: [Handle<Mesh>;3], motor_materials: [Handle<StandardMaterial>;3],
    smoke: Handle<StandardMaterial>,
}
#[derive(Component)]
pub struct GenericMissileDecoration(usize);
fn part(
    c: &mut Commands,
    parent: Entity,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    transform: Transform,
) -> Entity {
    let e = c
        .spawn((Mesh3d(mesh), MeshMaterial3d(material), transform))
        .id();
    c.entity(parent).add_child(e);
    e
}
fn missile_details(
    c: &mut Commands,
    root: Entity,
    seeker: &Handle<Mesh>,
    band: &Handle<Mesh>,
    dark: &Handle<StandardMaterial>,
    stripe: &Handle<StandardMaterial>,
    missile: Option<usize>,
) {
    let seeker_entity = part(
        c,
        root,
        seeker.clone(),
        dark.clone(),
        Transform::from_xyz(0.0, 0.0, -1.435)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    );
    if let Some(i) = missile { c.entity(seeker_entity).insert((GenericMissileDecoration(i), Visibility::Inherited)); }
    for (z, material) in [(-0.9, stripe), (0.57, dark)] {
        let band_entity = part(
            c,
            root,
            band.clone(),
            material.clone(),
            Transform::from_xyz(0.0, 0.0, z)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        );
        if let Some(i) = missile { c.entity(band_entity).insert((GenericMissileDecoration(i), Visibility::Inherited)); }
    }
}
pub fn setup(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    jet: Query<Entity, With<world::Jet>>,
) {
    let gray = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.49, 0.53),
        metallic: 0.4,
        perceptual_roughness: 0.6,
        cull_mode: None,
        ..default()
    });
    let orange = materials.add(StandardMaterial {
        base_color: Color::srgb(0.93, 0.29, 0.06),
        metallic: 0.1,
        cull_mode: None,
        ..default()
    });
    let dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.018, 0.035, 0.05),
        metallic: 0.7,
        perceptual_roughness: 0.15,
        ..default()
    });
    let white = materials.add(StandardMaterial {
        base_color: Color::srgb(0.82, 0.85, 0.86),
        metallic: 0.28,
        perceptual_roughness: 0.4,
        cull_mode: None,
        ..default()
    });
    let hot = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.54, 0.13, 0.65),
        emissive: LinearRgba::new(12.0, 6.0, 1.0, 1.0),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });
    let motor_materials = [
        materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.93, 0.65, 0.85),
            emissive: LinearRgba::new(8.0, 6.0, 2.0, 1.0),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgba(0.35, 0.62, 1.0, 0.38),
            emissive: LinearRgba::new(1.0, 2.0, 4.0, 1.0),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.48, 0.12, 0.25),
            emissive: LinearRgba::new(4.0, 1.8, 0.4, 1.0),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        }),
    ];
    let motor_meshes =
        [0.055, 0.082, 0.115].map(|radius| meshes.add(world::engine_surface(radius, 1.0, true, 1)));
    let smoke = materials.add(StandardMaterial {
        base_color: Color::srgba(0.93, 0.94, 0.96, 0.62),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });
    let body = meshes.add(body_mesh());
    let surfaces = meshes.add(surfaces_mesh());
    let rocket = meshes.add(missile_mesh());
    let radar_store = meshes.add(radar_visual::body(false,true));
    let aim9x = meshes.add(aim9x::body_mesh(false));
    let aim9x_assets = aim9x::prepare(&mut meshes,&mut materials,&mut images);
    let radar_assets = radar_visual::prepare(&mut meshes,&mut materials);
    c.insert_resource(MissileMeshes { generic: rocket.clone(), aim9x,
        motors: motor_meshes.clone(), motor_materials: motor_materials.clone(), smoke: smoke.clone() });
    let sphere = meshes.add(Sphere::new(1.0).mesh().uv(20, 12));
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let cone = meshes.add(Cone::new(0.14, 1.2));
    let seeker = meshes.add(Cone::new(0.077, 0.43));
    let band = meshes.add(Torus::new(0.080, 0.088));
    c.spawn((
        Button,
        CockpitIconsButton,
        Node {
            position_type: PositionType::Absolute,
            right: px(200),
            top: px(20),
            padding: UiRect::axes(px(12), px(7)),
            border_radius: BorderRadius::all(px(5)),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.025, 0.065, 0.075, 0.9)),
        ZIndex(50),
    )).with_children(|p| {
        p.spawn((CockpitIconsLabel, Text::new("ALT+H / ENEMY ICONS OFF"),
            TextFont { font_size: 12., ..default() },
            TextColor(Color::srgb(0.75, 0.93, 0.91))));
    });
    for i in 0..MAX_AIRCRAFT {
        let root = c
            .spawn((TargetVisual(i), Transform::default(), Visibility::Hidden))
            .id();
        crate::skin_damage::setup(&mut c, &mut meshes, &mut materials, root, Some(i));
        part(
            &mut c,
            root,
            body.clone(),
            gray.clone(),
            Transform::default(),
        );
        part(
            &mut c,
            root,
            surfaces.clone(),
            gray.clone(),
            Transform::default(),
        );
        for index in 0..9 {
            let (mesh, origin, axis) = target_control_mesh(index);
            let entity = part(
                &mut c,
                root,
                meshes.add(mesh),
                gray.clone(),
                Transform::from_translation(origin),
            );
            c.entity(entity).insert((
                TargetControl {
                    target: i,
                    index,
                    axis,
                },
                Visibility::Inherited,
            ));
        }
        part(
            &mut c,
            root,
            sphere.clone(),
            dark.clone(),
            Transform::from_xyz(0.0, 0.48, -2.95).with_scale(Vec3::new(0.46, 0.45, 1.05)),
        );
        for side in [-1.0, 1.0] {
            part(
                &mut c,
                root,
                cube.clone(),
                dark.clone(),
                Transform::from_xyz(side * 0.86, -0.10, -1.65)
                    .with_scale(Vec3::new(0.38, 0.48, 0.85)),
            );
            part(
                &mut c,
                root,
                cube.clone(),
                orange.clone(),
                Transform::from_xyz(side * 3.55, -0.11, 3.03)
                    .with_scale(Vec3::new(0.45, 0.035, 0.7)),
            );
        }
        part(
            &mut c,
            root,
            meshes.add(Cylinder::new(0.50, 0.65)),
            dark.clone(),
            Transform::from_xyz(0.0, 0.0, 5.65)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        );
        part(
            &mut c,
            root,
            cone.clone(),
            hot.clone(),
            Transform::from_xyz(0.0, 0.0, 6.4)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                .with_scale(Vec3::new(2.0, 1.0, 2.0)),
        );
    }
    for i in 0..RENDER_MISSILES {
        let root = c
            .spawn((MissileVisual(i), Mesh3d(rocket.clone()), MeshMaterial3d(white.clone()), Transform::default(), Visibility::Hidden))
            .id();
        missile_details(&mut c, root, &seeker, &band, &dark, &orange, Some(i));
        aim9x::install(&mut c,root,i,&aim9x_assets);
        radar_visual::install(&mut c,root,i,&radar_assets);
        for (layer, material) in motor_materials.iter().enumerate() {
            let flame = part(
                &mut c,
                root,
                motor_meshes[layer].clone(),
                material.clone(),
                Transform::from_xyz(0.0, 0.0, 2.0)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            );
            c.entity(flame).insert(MotorVisual(i, layer));
        }
        c.spawn((
            SmokeVisual(i),
            Mesh3d(meshes.add(mesh(vec![Vec3::ZERO, Vec3::X, Vec3::Y], vec![0, 1, 2]))),
            MeshMaterial3d(smoke.clone()),
            Transform::default(),
            Visibility::Hidden,
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
            Aabb::from_min_max(Vec3::ZERO, Vec3::ONE),
            // write_smoke_mesh supplies conservative bounds with the new vertices.
            bevy::camera::visibility::NoAutoAabb,
        ));
    }
    c.insert_resource(aim9x_assets);
    c.insert_resource(radar_assets);
    if let Some(jet) = jet.iter().next() {
        for (i, station) in STATIONS.iter().enumerate() {
            let pylon = part(
                &mut c,
                jet,
                cube.clone(),
                gray.clone(),
                Transform::from_translation(station.as_vec3() + Vec3::Y * 0.17)
                    .with_scale(Vec3::new(0.065, 0.30, 0.65)),
            );
            c.entity(pylon).insert(StoreVisual(i, true));
            let store = part(
                &mut c,
                jet,
                if i<2{rocket.clone()}else{radar_store.clone()},
                white.clone(),
                Transform::from_translation(station.as_vec3()),
            );
            c.entity(store).insert(StoreVisual(i, false));
            if i<2{missile_details(&mut c, store, &seeker, &band, &dark, &orange, None);}
        }
    }
    for i in 0..4 {
        let material = materials.add(StandardMaterial {
            base_color: Color::NONE,
            emissive: LinearRgba::BLACK,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });
        c.spawn((
            FlashVisual(i),
            Mesh3d(meshes.add(flash_mesh())),
            MeshMaterial3d(material),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    let panel = c
        .spawn((
            CombatPanel,
            Node {
                position_type: PositionType::Absolute,
                right: px(28),
                top: px(173),
                width: px(284),
                padding: UiRect::all(px(10)),
                border: UiRect::left(px(2)),
                border_radius: BorderRadius::all(px(7)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.055, 0.072, 0.91)),
            BorderColor::all(Color::srgb(0.30, 0.79, 0.74)),
            ZIndex(30),
        ))
        .id();
    let text = c
        .spawn((
            CombatReadout,
            Text::new(""),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(Color::srgb(0.74, 0.91, 0.91)),
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
    c.spawn((
        MissileReadout,
        Text::new(""),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.82, 0.94, 0.92)),
        Node {
            position_type: PositionType::Absolute,
            left: px(28),
            top: px(380),
            width: px(330),
            padding: UiRect::all(px(8)),
            border_radius: BorderRadius::all(px(6)),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.025, 0.055, 0.072, 0.92)),
        ZIndex(31),
    ));
    for (tag, label) in [(0, "TAB NEXT"), (1, "O TYPE"), (2, "SPACE FIRE")] {
        let button = c
            .spawn((
                Button,
                CombatButton(tag),
                Node {
                    width: px(84),
                    height: px(25),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.06, 0.19, 0.23)),
            ))
            .id();
        let text = c
            .spawn((
                Text::new(label),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgb(0.87, 0.95, 0.95)),
            ))
            .id();
        c.entity(button).add_child(text);
        c.entity(row).add_child(button);
    }
    for i in 0..MAX_AIRCRAFT {
        let marker = c
            .spawn((
                TargetMarker(i),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(28),
                    height: px(28),
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(Color::srgb(0.45, 0.95, 0.8)),
                Visibility::Hidden,
                ZIndex(29),
            ))
            .id();
        let text = c
            .spawn((
                TargetLabel(i),
                Text::new(""),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.95, 0.93)),
                Node {
                    position_type: PositionType::Absolute,
                    top: px(-24.0 - i as f32 * 18.0),
                    left: px(-35),
                    width: px(126),
                    padding: UiRect::all(px(3)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.045, 0.055, 0.78)),
            ))
            .id();
        c.entity(marker).add_child(text);
    }
}

pub fn controls(
    hangar:Option<Res<crate::hangar::Hangar>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut buttons: Query<(&Interaction, &CombatButton, &mut BackgroundColor), Changed<Interaction>>,
    f: Res<Flight>,
    v: Res<ViewState>,
    warp: Res<crate::warp::Warp>,
    mut combat: ResMut<Combat>,
    preview: Option<Res<crate::VisualCheck>>,
    environment: Option<Res<Environment>>,
    bvr_menu: Option<Res<bvr::Menu>>,
) {
    if combat.generation != v.generation {
        if v.scenario == 8 {
            bvr::start(
                &mut combat,
                &f,
                v.generation,
                &bvr_menu.as_deref().cloned().unwrap_or_default(),
            );
        } else if (4..=7).contains(&v.scenario) {
            combat.start(&f, v.generation);
            if v.scenario >= 5 {
                combat.start_air(&f, v.generation, v.scenario);
            }
        } else {
            *combat = Combat {
                generation: v.generation,
                ..default()
            };
        }
    }
    if let Some(environment) = environment {
        combat.sun = environment.sun_direction();
    }
    if hangar.as_ref().is_some_and(|h|h.open||h.block_input) || !combat.active
        || warp.editing
        || bvr_menu.as_ref().is_some_and(|m| m.open)
        || preview.is_some_and(|p| p.0.is_some())
    {
        combat.stop_gun();
        return;
    }
    let mut next = keys.just_pressed(KeyCode::Tab);
    let mut weapon = keys.just_pressed(KeyCode::KeyO)
        && !(f.aircraft == crate::aircraft::Aircraft::Hornet && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight)));
    let mut fire = keys.just_pressed(KeyCode::Space);
    for (interaction, button, mut color) in &mut buttons {
        color.0 = if *interaction == Interaction::None {
            Color::srgb(0.06, 0.19, 0.23)
        } else {
            Color::srgb(0.12, 0.32, 0.33)
        };
        if *interaction == Interaction::Pressed && !v.left_camera_drag {
            match button.0 {
                0 => next = true,
                1 => weapon = true,
                _ => fire = true,
            }
        }
    }
    if next {
        combat.cycle();
    }
    if weapon {
        combat.weapon = if combat.weapon == Weapon::Heat {
            Weapon::Radar
        } else {
            Weapon::Heat
        };
        combat.lock = 0.0;
    }
    if combat.cue(&f).is_err() {
        combat.lock = 0.0;
    }
    if fire {
        combat.fire(&f);
    }
    combat.defense_controls(&keys, &f);
}
pub fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    f: Res<Flight>,
    warp: Res<crate::warp::Warp>,
    mut view: ResMut<ViewState>,
    mut combat: ResMut<Combat>,
    preview: Option<Res<crate::VisualCheck>>,
) {
    if warp.editing {
        return;
    }
    let capture = preview.is_some_and(|p| p.0.is_some());
    if !capture && keys.just_pressed(KeyCode::Digit7) {
        view.restore_camera();
        return;
    }
    let aircraft: Vec<_> = combat
        .targets
        .iter()
        .enumerate()
        .filter(|(_, t)| combat.active && t.alive)
        .map(|(i, _)| i)
        .collect();
    if !capture
        && keys.just_pressed(KeyCode::Digit2)
        && !keys.pressed(KeyCode::AltLeft)
        && !keys.pressed(KeyCode::AltRight)
    {
        if aircraft.is_empty() {
            combat.notify("NO OTHER AIRCRAFT");
        } else {
            let next = view
                .aircraft_follow
                .and_then(|id| aircraft.iter().position(|&i| i == id))
                .map_or(0, |i| (i + 1) % aircraft.len());
            view.follow_aircraft(aircraft[next]);
            combat.notify(format!(
                "FOLLOW AIRCRAFT {:02} / 2 NEXT / 7 RETURN",
                aircraft[next] + 1
            ));
        }
        return;
    }
    if let Some(id) = view.aircraft_follow {
        if !aircraft.contains(&id) {
            if let Some(&next) = aircraft.first() {
                view.follow_aircraft(next);
            } else {
                view.restore_camera();
            }
        }
    }
    let airborne: Vec<_> = (0..RENDER_MISSILES)
        .filter(|&i| combat.render_missile(i).is_some_and(|m| m.alive))
        .collect();
    if !capture && keys.just_pressed(KeyCode::Digit6) && f.vehicle == Vehicle::Gripen {
        if airborne.is_empty() {
            view.restore_camera();
            combat.notify("NO AIRBORNE MISSILES");
        } else {
            let next = view
                .missile_follow
                .and_then(|follow| airborne.iter().position(|&id| id == follow.missile))
                .map_or(0, |index| (index + 1) % airborne.len());
            view.follow_missile(airborne[next]);
        }
    } else if let Some(follow) = view.missile_follow {
        if !airborne.contains(&follow.missile) {
            if let Some(&missile) = airborne.first() {
                view.follow_missile(missile);
            } else {
                view.restore_camera();
                combat.notify("MISSILE FLIGHT ENDED / PLAYER CAMERA");
            }
        }
    }
}

pub fn render(
    f: Res<Flight>,
    combat: Res<Combat>,
    missile_meshes: Res<MissileMeshes>,
    aim9x_assets: Res<aim9x::VisualAssets>,
    radar_assets: Res<radar_visual::Assets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cameras: Query<&GlobalTransform, With<FlightCamera>>,
    mut decorations: Query<(&GenericMissileDecoration, &mut Visibility), (
        Without<TargetVisual>, Without<MissileVisual>, Without<MotorVisual>,
        Without<FlashVisual>, Without<StoreVisual>)>,
    mut targets: Query<
        (&TargetVisual, &mut Transform, &mut Visibility),
        (
            Without<MissileVisual>,
            Without<MotorVisual>,
            Without<FlashVisual>,
            Without<StoreVisual>,
        ),
    >,
    mut missiles: Query<
        (&MissileVisual, &mut Transform, &mut Visibility, &mut Mesh3d),
        (
            Without<TargetVisual>,
            Without<MotorVisual>,
            Without<FlashVisual>,
            Without<StoreVisual>,
        ),
    >,
    mut motors: Query<
        (&MotorVisual, &mut Transform, &mut Visibility, &mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>),
        (
            Without<TargetVisual>,
            Without<MissileVisual>,
            Without<FlashVisual>,
            Without<StoreVisual>,
        ),
    >,
    mut stores: Query<
        (&StoreVisual, &mut Visibility),
        (
            Without<TargetVisual>,
            Without<MissileVisual>,
            Without<MotorVisual>,
            Without<FlashVisual>,
        ),
    >,
    mut flashes: Query<
        (
            &FlashVisual,
            &mut Transform,
            &mut Visibility,
            &MeshMaterial3d<StandardMaterial>,
        ),
        (
            Without<TargetVisual>,
            Without<MissileVisual>,
            Without<MotorVisual>,
            Without<StoreVisual>,
        ),
    >,
) {
    for (tag, mut visibility) in &mut decorations {
        let desired = if combat.render_missile(tag.0).is_some_and(|m|m.gnc.aim9x||m.kind==Weapon::Radar) {
            Visibility::Hidden
        } else { Visibility::Inherited };
        if *visibility != desired { *visibility = desired; }
    }
    for (tag, mut transform, mut visibility) in &mut targets {
        let target = combat
            .targets
            .get(tag.0)
            .filter(|t| combat.active && t.alive);
        visibility.set_if_neq(if target.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if let Some(t) = target {
            let mut desired=*transform;
            desired.translation = world::local(t.position, &f);
            desired.rotation = world::local_rot(t.orientation, &f);
            transform.set_if_neq(desired);
        }
    }
    for (tag, mut transform, mut visibility, mut mesh) in &mut missiles {
        let missile = combat
            .render_missile(tag.0)
            .filter(|m| combat.active && m.alive);
        visibility.set_if_neq(if missile.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if let Some(m) = missile {
            let handle = if m.gnc.aim9x { &missile_meshes.aim9x }
                else if m.kind==Weapon::Radar {&radar_assets.bodies[usize::from(m.gnc.long_range)]}
                else { &missile_meshes.generic };
            if mesh.0 != *handle { mesh.0 = handle.clone(); }
            let mut desired=*transform;
            desired.translation = world::local(m.position, &f);
            let nose = if m.gnc.body.length_squared() > 0.5 {
                m.gnc.body
            } else {
                m.velocity.normalize()
            };
            let attitude=DQuat::from_rotation_arc(m.gnc.attitude*-DVec3::Z,nose)*m.gnc.attitude;
            desired.rotation = world::local_rot(attitude, &f);
            desired.scale = Vec3::ONE;
            transform.set_if_neq(desired);
        }
    }
    for (tag, mut transform, mut visibility, mut mesh, mut material) in &mut motors {
        let missile = combat
            .render_missile(tag.0)
            .filter(|m| combat.active && m.alive && m.gnc.thrust > 0.0);
        visibility.set_if_neq(if missile.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if let Some(m) = missile {
            let (mesh_handle,mat_handle)=if m.gnc.aim9x || m.kind==Weapon::Radar {
                (&aim9x_assets.plumes[tag.1],&aim9x_assets.plume_materials[tag.1])
            }else{(&missile_meshes.motors[tag.1],&missile_meshes.motor_materials[tag.1])};
            if mesh.0!=*mesh_handle{mesh.0=mesh_handle.clone();}
            if material.0!=*mat_handle{material.0=mat_handle.clone();}
            if m.gnc.aim9x {
                transform.set_if_neq(aim9x::plume_transform(m,tag.1));
                continue;
            }
            if m.kind==Weapon::Radar {
                transform.set_if_neq(radar_visual::plume(m,tag.1));
                continue;
            }
            transform.rotation=Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
            let ignition = ((m.age - 0.18) / 0.16).clamp(0.0, 1.0) as f32;
            let pulse = 1.0 + 0.055 * (m.age as f32 * 47.0).sin();
            let length = [1.8, 2.4, 3.0][tag.1] * ignition * pulse;
            let width = [0.75, 1.20, 1.65][tag.1] * ignition;
            transform.scale = Vec3::new(width, length, width);
            transform.translation.z = 1.38 + length * 0.5;
        }
    }
    for (tag, mut visibility) in &mut stores {
        let kind = tag.0 / 2;
        let round = tag.0 % 2;
        visibility.set_if_neq(if combat.active
            && (tag.1 || round >= 2_usize.saturating_sub(combat.ammo[kind] as usize))
        {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    for (tag, mut transform, mut visibility, handle) in &mut flashes {
        let impact = combat.impacts.get(tag.0).filter(|_| combat.active);
        visibility.set_if_neq(if impact.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if let Some(i) = impact {
            transform.translation = world::local(i.position, &f);
            transform.scale = Vec3::splat((2.0 + i.age * 13.0) as f32);
            let toward_eye = cameras.iter().next().map_or(Vec3::Z, |camera| {
                (camera.translation() - transform.translation).normalize_or_zero()
            });
            transform.rotation = Quat::from_rotation_arc(Vec3::Z, toward_eye);
            if let Some(mat) = materials.get_mut(&handle.0) {
                let fade = (1.0 - i.age / 4.0).max(0.0) as f32;
                let heat = (1.0 - i.age / 0.9).max(0.0) as f32;
                mat.base_color = Color::srgba(
                    0.22 + 0.78 * heat,
                    0.24 + 0.36 * heat,
                    0.26 - 0.14 * heat,
                    fade * 0.9,
                );
                mat.emissive =
                    LinearRgba::new(8.0 * fade * heat, 2.0 * fade * heat, 0.3 * fade * heat, 1.0);
            }
        }
    }
}

/// Impact flash sphere with its eye-facing glow baked for an eye along +Z.
/// The flash is rotated to face the camera instead of rewriting and
/// re-uploading every vertex colour each frame; the material is unlit, so only
/// the colour pattern depends on orientation.
fn flash_mesh() -> Mesh {
    let mut mesh = Sphere::new(1.0).mesh().uv(32, 20);
    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
        mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
    {
        let colors: Vec<_> = normals
            .iter()
            .map(|normal| {
                let glow = Vec3::from(*normal).dot(Vec3::Z).max(0.0);
                [1.0, 0.45 + 0.55 * glow, 0.20 + 0.65 * glow, glow.powf(1.6)]
            })
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    mesh
}

pub fn draw_telemetry(
    f: Res<Flight>,
    combat: Res<Combat>,
    v: Res<ViewState>,
    mut texts: Query<(&mut Text, &mut Node), With<MissileReadout>>,
) {
    let tracked = v
        .missile_follow
        .and_then(|follow| combat.render_missile(follow.missile))
        .or_else(|| combat.missiles.iter().rev().find(|m| m.alive));
    let active = combat.active
        && v.aircraft_follow.is_none()
        && !v.map
        && (!v.simple || v.missile_follow.is_some())
        && (v.index != 3 || v.missile_follow.is_some())
        && f.vehicle == Vehicle::Gripen;
    for (mut text, mut node) in &mut texts {
        if let Some(m) = tracked.filter(|m| m.alive && active) {
            if node.display != Display::Flex {
                node.display = Display::Flex;
            }
            let (_, _, _, sound) = atmosphere(m.position.length() - RADIUS);
            let r = m.gnc.estimate - m.position;
            let vc = (m.velocity - m.gnc.estimated_velocity).dot(r.normalize_or_zero());
            let eta = if m.gnc.initialized && vc > 10.0 && m.guided {
                format!("CLOSING ETA {:.1}s", (r.length() / vc).min(999.0))
            } else {
                "NO CLOSING SOLUTION".into()
            };
            let instruction = match m.gnc.mode {
                guidance::Mode::Supported => {
                    if combat.bvr.is_some() {
                        "MAINTAIN TWS TRACK / STT SUPPORT"
                    } else {
                        "KEEP TARGET SELECTED / RADAR SUPPORT"
                    }
                }
                guidance::Mode::Memory => "TRACK LOST / PREDICTING",
                guidance::Mode::Ballistic => "GUIDANCE LOST / BALLISTIC",
                guidance::Mode::Decoy => "DECOY TRACK / COMPROMISED",
                _ => m.gnc.reason,
            };
            let readout = format!(
                "MISSILE / {} · SIGNAL {:.0}%\n{} · M {:.2} · {:.0} m/s\nTURN {:.1}/{:.1} G · FUEL {:.1} kg\n{}\n{}\n{}",
                if m.kind == Weapon::Heat && m.gnc.mode == guidance::Mode::Supported {
                    "IR / RADAR DATALINK"
                } else {
                    m.gnc.mode.label()
                },
                m.gnc.signal * 100.0,
                if m.gnc.thrust > 0.0 { "BOOST" } else { "COAST" },
                m.velocity.length() / sound,
                m.velocity.length(),
                m.gnc.actual_g,
                m.gnc.available_g,
                m.gnc.propellant,
                if m.gnc.loft_blend > 0.01 {
                    format!(
                        "LOFT / PATH {:.1} km · BLEND {:.0}%",
                        m.gnc.loft_altitude / 1000.0,
                        m.gnc.loft_blend * 100.0
                    )
                } else {
                    "DIRECT / TERMINAL GUIDANCE".into()
                },
                eta,
                instruction
            );
            // Unchanged text assigned through Mut still forces a re-layout.
            if text.0 != readout {
                text.0 = readout;
            }
        } else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            if !text.0.is_empty() {
                text.0.clear();
            }
        }
    }
}
pub fn draw(
    f: Res<Flight>,
    v: Res<ViewState>,
    combat: Res<Combat>,
    camera: Query<(&Camera, &GlobalTransform), With<FlightCamera>>,
    mut panels: Query<&mut Node, (With<CombatPanel>, Without<TargetMarker>)>,
    mut markers: Query<
        (&TargetMarker, &mut Node, &mut BorderColor, &mut Visibility),
        Without<CombatPanel>,
    >,
    mut texts: Query<(&mut Text, Option<&TargetLabel>, Option<&CombatReadout>)>,
    mut gizmos: Gizmos,
) {
    let active = combat.active && combat.bvr.is_none() && f.vehicle == Vehicle::Gripen && !v.map;
    for mut panel in &mut panels {
        panel.display = if active && !v.simple {
            Display::Flex
        } else {
            Display::None
        };
    }
    let camera = camera.iter().next();
    for (tag, mut node, mut border, mut visibility) in &mut markers {
        *visibility = Visibility::Hidden;
        if !target_marker_visible(&combat, &f, &v, tag.0) {
            continue;
        }
        if let (Some(t), Some((camera, transform))) =
            (combat.targets.get(tag.0).filter(|t| t.alive), camera)
        {
            if v.index == 3 && !guidance::terrain_visible(f.position, t.position) {
                continue;
            }
            if let Ok(p) = camera.world_to_viewport(transform, world::local(t.position, &f)) {
                if let Some(size) = camera.logical_viewport_size() {
                    let top = if v.index == 3 { 45. } else { 165. };
                    let bottom = if v.index == 3 { 30. } else { 240. };
                    let (left, right) = if v.index == 3 { (45., 95.) } else { (20., 150.) };
                    if p.x < left || p.x > size.x - right || p.y < top || p.y > size.y - bottom {
                        continue;
                    }
                }
                node.left = px(p.x - 14.0);
                node.top = px(p.y - 14.0);
                *border = BorderColor::all(if tag.0 == combat.selected {
                    Color::srgb(1.0, 0.73, 0.30)
                } else {
                    if v.index == 3 { Color::srgb(1., 0.48, 0.40) }
                        else { Color::srgb(0.45, 0.65, 0.72) }
                });
                *visibility = Visibility::Inherited;
            }
        }
    }
    for (mut text, label, readout) in &mut texts {
        if let Some(label) = label {
            if label.0 == combat.selected && v.missile_follow.is_none() && v.index != 3 {
                if !text.0.is_empty() {
                    text.0.clear();
                }
                continue;
            }
            if let Some(t) = combat.targets.get(label.0) {
                let readout = format!(
                    "T{:02} / {:.2} km",
                    label.0 + 1,
                    t.position.distance(f.position) / 1000.0
                );
                if text.0 != readout {
                    text.0 = readout;
                }
            }
        } else if readout.is_some() && active {
            let range = combat
                .targets
                .get(combat.selected)
                .map_or(0.0, |t| t.position.distance(f.position));
            let direction = combat.targets.get(combat.selected).map_or(-DVec3::Z, |t| {
                f.orientation.conjugate() * (t.position - f.position).normalize_or_zero()
            });
            let azimuth = direction.x.atan2(-direction.z).to_degrees();
            let elevation = direction.y.clamp(-1.0, 1.0).asin().to_degrees();
            let state = if combat.hits == TARGET_COUNT {
                "ALL TARGETS DESTROYED / R RESTART".into()
            } else if combat.clock < combat.message_until {
                combat.message.clone()
            } else if let Err(reason) = combat.cue(&f) {
                reason.into()
            } else if combat.ammo[combat.weapon.index()] == 0 {
                "EMPTY / O CHANGE WEAPON".into()
            } else if combat.lock >= combat.weapon.lock_time() {
                "LOCKED / FIRE AUTHORIZED".into()
            } else {
                format!(
                    "ACQUIRING {:.0}%",
                    100.0 * combat.lock / combat.weapon.lock_time()
                )
            };
            let readout = format!(
                "{} / STRIKES {} · DOWN {}/{}\n{} / IR {} · RAD {}\nT{:02} / {:.2} km  · Vc {:+.0} m/s\nNOSE AZ {:+.0}° · EL {:+.0}°\n{}",
                if combat.difficulty().is_some() {
                    "COMBAT"
                } else {
                    "PRACTICE"
                },
                combat.strikes,
                combat.hits,
                TARGET_COUNT,
                combat.weapon.label(),
                combat.ammo[0],
                combat.ammo[1],
                combat.selected + 1,
                range / 1000.0,
                combat
                    .targets
                    .get(combat.selected)
                    .map_or(0.0, |t| hud::closing(&f, t)),
                azimuth,
                elevation,
                state
            );
            if text.0 != readout {
                text.0 = readout;
            }
        }
    }
    if !active {
        return;
    }
    for m in combat.missiles_for_draw() {
        if m.alive && !m.gnc.aim9x && m.kind!=Weapon::Radar {
            for (i, pair) in m.trail.windows(2).enumerate() {
                let alpha = 0.08 + 0.4 * i as f32 / m.trail.len().max(1) as f32;
                gizmos.line(
                    world::local(pair[0], &f),
                    world::local(pair[1], &f),
                    Color::srgba(0.94, 0.93, 0.85, alpha),
                );
            }
        }
    }
    for impact in &combat.impacts {
        if impact.hit {
            for i in 0..9 {
                let a = i as f64 * 2.39996;
                let direction = DVec3::new(a.cos(), 0.4 + (i % 3) as f64 * 0.35, a.sin());
                let p = impact.position + direction * (impact.age * 35.0)
                    - impact.position.normalize() * 4.9 * impact.age.powi(2);
                gizmos.line(
                    world::local(p, &f),
                    world::local(p - direction * 3.0, &f),
                    Color::srgba(1.0, 0.55, 0.1, (1.0 - impact.age / 4.0) as f32),
                );
            }
        }
    }
}

pub fn smoke(
    f: Res<Flight>,
    combat: Res<Combat>,
    aim9x_assets: Res<aim9x::VisualAssets>,
    missile_meshes: Res<MissileMeshes>,
    cameras: Query<&Transform, With<FlightCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut smoke: Query<(&SmokeVisual, &Mesh3d, &mut MeshMaterial3d<StandardMaterial>, &mut Visibility, &mut Aabb, &mut Transform), Without<FlightCamera>>,
    mut previous: Local<Vec<Option<TrailState>>>,
) {
    previous.resize(RENDER_MISSILES, None);
    let basis = world::frame(&f).conjugate();
    let origin = world::render_origin(&f);
    let camera=cameras.iter().next().copied().unwrap_or_default();
    for (tag, handle, mut material, mut visibility, mut bounds, mut transform) in &mut smoke {
        let missile = combat.render_missile(tag.0).filter(|m| {
            combat.active
                && m.alive
                && !(smoke_gpu::enabled() && (m.gnc.aim9x || m.kind == Weapon::Radar))
                && m.trail.len() > 2
                && m.age < (if m.gnc.long_range { 24. } else { m.kind.burn() }) + 3.8
        });
        visibility.set_if_neq(if missile.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let Some(missile) = missile else {
            previous[tag.0] = None;
            continue;
        };
        let detailed=missile.gnc.aim9x||missile.kind==Weapon::Radar;
        let mat=if detailed{&aim9x_assets.smoke}else{&missile_meshes.smoke};
        if material.0!=*mat{material.0=mat.clone();}
        // Generic trails are built relative to their first point in world
        // axes and placed by Transform, so a coasting trail keeps its vertex
        // buffer while the render origin follows the aircraft. Camera-facing
        // detailed trails stay in camera-local space.
        let anchor = missile.trail[0];
        let placed = if detailed {
            Transform::default()
        } else {
            Transform::from_translation((basis * (anchor - origin)).as_vec3())
                .with_rotation(basis.as_quat())
        };
        if *transform != placed {
            *transform = placed;
        }
        let state = TrailState {
            basis: if detailed { basis } else { DQuat::IDENTITY },
            origin: if detailed { origin } else { anchor },
            // A generic trail's vertices depend only on its points and, while
            // powered, the nozzle tail; the detailed cards also track the missile.
            age: if detailed { missile.age } else { 0. },
            position: if detailed || missile.gnc.thrust > 0. || smoke_clearance_live(missile) {
                missile.position
            } else {
                *missile.trail.last().unwrap()
            },
            velocity: if detailed {
                missile.velocity
            } else if missile.gnc.thrust > 0. {
                missile.gnc.body
            } else {
                DVec3::ZERO
            },
            powered: missile.gnc.thrust > 0.,
            points: missile.trail.len(),
            generation: combat.generation,
            camera: detailed.then_some((camera.translation, camera.rotation)),
        };
        if previous[tag.0] == Some(state) {
            continue;
        }
        if let Some(asset) = meshes.get_mut(&handle.0) {
            if detailed {
                *bounds = aim9x::write_smoke(
                    asset,
                    missile,
                    basis,
                    origin,
                    camera.translation,
                    camera.rotation * Vec3::Y,
                );
                previous[tag.0] = Some(state);
                continue;
            }
            let tail = state.powered.then(|| {
                missile.position - missile.gnc.body.normalize_or(missile.velocity.normalize()) * 1.6
            });
            *bounds = write_smoke_mesh(
                asset,
                &missile.trail,
                tail,
                missile.position,
                DQuat::IDENTITY,
                anchor,
            );
            previous[tag.0] = Some(state);
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct TrailState {
    basis: DQuat,
    origin: DVec3,
    age: f64,
    position: DVec3,
    velocity: DVec3,
    powered: bool,
    points: usize,
    generation: u64,
    camera: Option<(Vec3, Quat)>,
}
/// Smoke stays clear of the first metres behind the nozzle and fades in over
/// a ramp, so the missile (and a chase camera ~13 m behind it) is not buried
/// in its own fresh exhaust.
pub(crate) const SMOKE_CLEAR_M: f64 = 16.0;
pub(crate) const SMOKE_RAMP_M: f64 = 14.0;
/// Opacity factor for smoke `distance` metres behind the nozzle.
pub(crate) fn smoke_clearance(distance: f64) -> f32 {
    let t = ((distance - SMOKE_CLEAR_M) / SMOKE_RAMP_M).clamp(0.0, 1.0) as f32;
    t * t * (3.0 - 2.0 * t)
}
/// Distance of each smoke point from the nozzle, along the trail: the gap
/// from the nozzle to the newest point plus arc length back from there.
pub(crate) fn smoke_distances(points: &[DVec3], nozzle: DVec3) -> Vec<f64> {
    let mut d = vec![0.0; points.len()];
    let Some(last) = points.len().checked_sub(1) else {
        return d;
    };
    d[last] = nozzle.distance(points[last]);
    for i in (0..last).rev() {
        d[i] = d[i + 1] + points[i].distance(points[i + 1]);
    }
    d
}
/// True while the nozzle is still close enough to the trail end for the
/// clearance ramp to depend on where the missile currently is.
pub(crate) fn smoke_clearance_live(m: &Missile) -> bool {
    m.trail
        .last()
        .is_some_and(|p| p.distance(m.position) < SMOKE_CLEAR_M + SMOKE_RAMP_M + 2.0)
}
fn write_smoke_mesh(
    asset: &mut Mesh,
    trail: &[DVec3],
    tail: Option<DVec3>,
    nozzle: DVec3,
    basis: DQuat,
    origin: DVec3,
) -> Aabb {
    use bevy::mesh::VertexAttributeValues;
    let point_count = trail.len() + usize::from(tail.is_some());
    let segments = point_count.saturating_sub(1);
    let vertices = segments * 20;
    if !asset.contains_attribute(Mesh::ATTRIBUTE_COLOR) {
        asset.insert_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new());
    }
    // This material is unlit and casts/receives no shadows: its normals are not
    // consumed by shading. Preserve every vertex/color/index, skip normal sums.
    for (attribute, values) in asset.attributes_mut() {
        if attribute.id == Mesh::ATTRIBUTE_NORMAL.id {
            if let VertexAttributeValues::Float32x3(v) = values {
                v.resize(vertices, [0., 0., 1.]);
            }
        } else if attribute.id == Mesh::ATTRIBUTE_UV_0.id {
            if let VertexAttributeValues::Float32x2(v) = values {
                v.resize(vertices, [0., 0.]);
            }
        }
    }
    if !matches!(asset.indices(), Some(Indices::U32(_))) {
        asset.insert_indices(Indices::U32(Vec::new()));
    }
    if let Some(Indices::U32(indices)) = asset.indices_mut() {
        if indices.len() != segments * 48 {
            indices.clear();
            for section in 0..segments * 2 {
                let n = section as u32 * 10;
                for j in 0..4 {
                    let k = n + j;
                    indices.extend([k, k + 1, k + 5, k + 1, k + 6, k + 5]);
                }
            }
        }
    }
    let (mut positions, mut colors) = (None, None);
    for (attribute, values) in asset.attributes_mut() {
        if attribute.id == Mesh::ATTRIBUTE_POSITION.id {
            if let VertexAttributeValues::Float32x3(v) = values {
                positions = Some(v);
            }
        } else if attribute.id == Mesh::ATTRIBUTE_COLOR.id {
            if let VertexAttributeValues::Float32x4(v) = values {
                colors = Some(v);
            }
        }
    }
    let positions = positions.expect("smoke positions");
    let colors = colors.expect("smoke colors");
    positions.clear();
    colors.clear();
    let points = trail.iter().copied().chain(tail);
    let all: Vec<DVec3> = points.clone().collect();
    let distance = smoke_distances(&all, tail.unwrap_or(nozzle));
    for (i, (a, b)) in points.clone().zip(points.skip(1)).enumerate() {
        let tangent = (b - a).normalize_or_zero();
        let up = a.normalize();
        let right = tangent.cross(up).normalize_or_zero();
        let centers = [a, b].map(|point| (basis * (point - origin)).as_vec3());
        for axis in [up, right] {
            for (end, center) in centers.into_iter().enumerate() {
                let fraction = (i + end) as f32 / (point_count - 1) as f32;
                let width =
                    (0.12 + 1.7 * (1.0 - fraction)) * (1.0 + 0.10 * ((i + end) as f32 * 1.7).sin());
                let alpha = (fraction * 4.0).min(1.0) * 0.75 * smoke_clearance(distance[i + end]);
                for (across, feather) in [
                    (-1.0, 0.0),
                    (-0.5, 0.45),
                    (0.0, 1.0),
                    (0.5, 0.45),
                    (1.0, 0.0),
                ] {
                    positions.push((center + axis.as_vec3() * width * across).to_array());
                    colors.push([1.0, 1.0, 1.0, alpha * feather]);
                }
            }
        }
    }
    let mut bounds = Aabb::enclosing(positions.iter().map(|p| Vec3::from(*p)))
        .unwrap_or_else(|| Aabb::from_min_max(Vec3::ZERO, Vec3::ZERO));
    // Allow for center/half-extent rounding at distant float render coordinates.
    bounds.half_extents +=
        bounds.center.abs() * (4. * f32::EPSILON) + bevy::math::Vec3A::splat(0.01);
    bounds
}

/// Close inspection poses only for the disposable screenshot process.
pub fn capture_camera(
    preview: Res<crate::VisualCheck>,
    f: Res<Flight>,
    combat: Res<Combat>,
    mut cameras: Query<&mut Transform, With<FlightCamera>>,
) {
    let Some(mode) = preview.0.as_deref() else {
        return;
    };
    let focus = match mode {
        "combat-launch" | "combat-flight" | "combat-vector" => combat
            .missiles
            .first()
            .map(|m| (m.position, m.velocity.normalize(), 12.0)),
        "combat-target" | "air-target-damage" | "air-component-collision" => combat
            .targets
            .first()
            .map(|t| (t.position, t.velocity.normalize(), 28.0)),
        "combat-impact" => combat
            .impacts
            .first()
            .map(|i| (i.position, f.forward(), 65.0)),
        _ => None,
    };
    if let Some((center, forward, distance)) = focus {
        let up = center.normalize();
        let right = forward.cross(up).normalize();
        let close=std::env::var("NORD_MISSILE_CLOSE").ok();
        let eye = if close.as_deref()==Some("rear") { center-forward*3.7+right*1.0+up*0.4 }
            else if close.as_deref()==Some("detail"){center+right*4.0+forward*0.7+up*0.7}
            else if close.as_deref()==Some("plume"){center+right*5.0-forward*1.5+up*0.6}
            else{center + right * distance + forward * (distance * 0.45) + up * (distance * 0.23)};
        let look = center - forward * if close.is_some(){0.0}else if mode == "combat-flight" { 7.0 } else { 1.0 };
        for mut transform in &mut cameras {
            *transform = Transform::from_translation(world::local(eye, &f)).looking_at(
                world::local(look, &f),
                (world::frame(&f).conjugate() * up).as_vec3(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cockpit_icons_include_selected_enemies_in_simple_and_bvr_views() {
        let f = flight();
        let mut c = Combat::default();
        c.start(&f, 1);
        let mut v = ViewState { index: 3, simple: true, ..default() };
        assert!(!target_marker_visible(&c, &f, &v, c.selected));
        v.cockpit_enemy_icons = true;
        assert!(target_marker_visible(&c, &f, &v, c.selected));
        c.targets[1].alive = false;
        assert!(!target_marker_visible(&c, &f, &v, 1));
        v.map = true;
        assert!(!target_marker_visible(&c, &f, &v, 0));
        v.map = false;
        v.follow_aircraft(0);
        assert!(!target_marker_visible(&c, &f, &v, 0));
        v.restore_camera();
        let mut menu = bvr::Menu::default();
        menu.formation = 1;
        bvr::start(&mut c, &f, 1, &menu);
        assert!(target_marker_visible(&c, &f, &v, 0));
        assert!(!target_marker_visible(&c, &f, &v, 2));
    }
    #[test]
    fn cockpit_icon_button_toggles_once_per_click_and_shortcut_works_paused() {
        let mut app = App::new();
        let mut f = flight();
        f.paused = true;
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(f)
            .insert_resource(ViewState { index: 3, ..default() })
            .add_systems(Update, cockpit_icon_controls);
        let button = app.world_mut().spawn((CockpitIconsButton, Interaction::Pressed,
            Node::default(), BackgroundColor::default())).id();
        app.update();
        assert!(app.world().resource::<ViewState>().cockpit_enemy_icons);
        app.update();
        assert!(app.world().resource::<ViewState>().cockpit_enemy_icons);
        *app.world_mut().get_mut::<Interaction>(button).unwrap() = Interaction::None;
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::AltLeft);
        keys.press(KeyCode::KeyH);
        app.update();
        assert!(!app.world().resource::<ViewState>().cockpit_enemy_icons);
    }
    #[test]
    fn f22_launch_uses_aim9x_and_wider_acquisition_cone() {
        for a in crate::aircraft::Aircraft::ALL {
            let mut f = flight();
            crate::aircraft::select(&mut f, a);
            f.systems.bays = 1.0;
            f.systems.side_bays=[crate::aircraft::bays::State{door:1.,launcher:1.};2];
            let mut c = Combat::default();
            c.start(&f, 1);
            c.lock = Weapon::Heat.lock_time();
            assert!(c.fire(&f));
            assert_eq!(c.missiles[0].gnc.aim9x, matches!(a,crate::aircraft::Aircraft::F22|crate::aircraft::Aircraft::Hornet));
            c.targets[0].position = f.position + f.orientation * DVec3::new(1500.0, 0.0, -1500.0);
            assert_eq!(c.cue(&f).is_ok(), matches!(a,crate::aircraft::Aircraft::F22|crate::aircraft::Aircraft::Hornet));
        }
    }
    #[test]
    fn hornet_releases_two_9x_and_six_amraams_from_matching_racks() {
        let mut f=flight();crate::aircraft::select(&mut f,crate::aircraft::Aircraft::Hornet);let mut c=Combat::default();c.start(&f,1);f.rates=DVec3::ZERO;
        for i in 0..8 {
            c.weapon=if i<2{Weapon::Heat}else{Weapon::Radar};c.lock=2.;c.clock+=1.;let rack=crate::aircraft::station_for(&f,i);
            assert!(c.fire(&f),"failed station {i}");let m=c.missiles.last().unwrap();assert_eq!(m.gnc.aim9x,i<2);assert!(m.position.distance(f.position+f.orientation*rack)<1e-8);
            if i<2 {assert!((m.velocity-f.velocity-f.forward()*18.).length()<1e-8);}
            f.systems.stores[i]=false;
        }
        assert_eq!(c.ammo,[0,0]);assert!(!c.fire(&f));
        assert_eq!(crate::aircraft::missile_name(f.aircraft,Weapon::Heat),"AIM-9X");assert_eq!(crate::aircraft::missile_name(f.aircraft,Weapon::Radar),"AIM-120");
    }
    #[test]
    fn aim9x_mesh_matches_public_envelope() {
        use bevy::mesh::VertexAttributeValues;
        let mesh = aim9x_mesh();
        let Some(VertexAttributeValues::Float32x3(vertices)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { panic!("missing vertices") };
        let min_z = vertices.iter().map(|p|p[2]).fold(f32::INFINITY, f32::min);
        let max_z = vertices.iter().map(|p|p[2]).fold(f32::NEG_INFINITY, f32::max);
        let span = vertices.iter().map(|p|p[0].abs().max(p[1].abs())).fold(0_f32, f32::max)*2.0;
        assert!((max_z-min_z-3.02).abs()<1e-5);
        assert!((span-0.45).abs()<1e-5);
    }
    #[test]
    fn fleet_inventory_launches_every_station_without_two_round_assumption(){
        for a in crate::aircraft::Aircraft::ALL {let mut f=flight();crate::aircraft::select(&mut f,a);f.systems.bays=1.;f.systems.main_stroke=[1.;6];f.systems.side_bays=[crate::aircraft::bays::State{door:1.,launcher:1.};2];let mut c=Combat::default();c.start(&f,1);
            assert_eq!(c.ammo,a.spec().ammo);
            for kind in [Weapon::Heat,Weapon::Radar]{c.weapon=kind;let n=c.ammo[kind.index()];for _ in 0..n {c.lock=kind.lock_time();c.clock+=1.;assert!(c.fire(&f),"{a:?} {kind:?}: {}",c.message);let m=c.missiles.last().unwrap();assert!(m.position.distance(f.position)<a.spec().length);assert!(m.position.is_finite());let index=c.missiles.len()-1;assert!(std::ptr::eq(c.render_missile(index).expect("every launch must be visible"),m));}assert!(!c.fire(&f));}
            assert_eq!(c.ammo,[0,0]);assert_eq!(c.missiles.len(),(a.spec().ammo[0]+a.spec().ammo[1])as usize);assert!(c.targets.iter().all(|t|t.dynamics.aircraft==crate::aircraft::Aircraft::Gripen));
        }
    }
    #[test]
    fn f22_air_combat_renders_all_six_radar_launches_and_preserves_enemy_slots() {
        let mut f = flight();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::F22);
        let mut c = Combat::default();
        for scenario in 5..=7 {
            // Restart at the same generation: inventory belongs to scenario
            // initialization, not a later hangar generation check.
            c.start_air(&f, 1, scenario);
            assert_eq!(c.ammo, [2, 6]);
            assert_eq!(c.fight.as_ref().unwrap().gun, f.aircraft.spec().gun);
            c.targets[0].position = f.position + f.forward() * 3500.;
            for _ in 0..6 {
                // This inventory/render test isolates a mechanism already at
                // its release pose. The queue tests exercise the full sweep.
                f.systems.bays=1.;f.systems.main_stroke=[1.;6];
                c.lock = Weapon::Radar.lock_time();
                c.clock += 1.;
                assert!(c.fire(&f), "{}", c.message);
                let index = c.missiles.len() - 1;
                assert!(std::ptr::eq(c.render_missile(index).unwrap(), &c.missiles[index]));
            }
            c.fight.as_mut().unwrap().enemy_missiles.push(c.missiles[0].clone());
            assert!(std::ptr::eq(c.render_missile(MISSILE_COUNT).unwrap(),
                &c.fight.as_ref().unwrap().enemy_missiles[0]));
            assert!(!c.fire(&f));
            assert_eq!(c.ammo, [2, 0]);
            assert_eq!(c.fired, 6);
        }
    }
    #[test]
    fn smoke_starts_clear_behind_the_nozzle_and_fades_in() {
        assert_eq!(smoke_clearance(0.0), 0.0);
        assert_eq!(smoke_clearance(SMOKE_CLEAR_M), 0.0);
        assert!(smoke_clearance(SMOKE_CLEAR_M + SMOKE_RAMP_M * 0.5) > 0.4);
        assert_eq!(smoke_clearance(SMOKE_CLEAR_M + SMOKE_RAMP_M), 1.0);
        let points = [DVec3::ZERO, DVec3::Z * 10.0, DVec3::Z * 20.0];
        let d = smoke_distances(&points, DVec3::Z * 25.0);
        assert_eq!(d, vec![25.0, 15.0, 5.0]);
    }
    #[test]
    fn streamed_trails_preserve_vertices_colors_topology_and_conservative_bounds() {
        use bevy::mesh::VertexAttributeValues;
        for f in [Flight::runway(), Flight::spacecraft()] {
            for count in [3, 13, 48] {
                let mut trail: Vec<_> = (0..count)
                    .map(|i| {
                        f.position
                            + DVec3::new(
                                i as f64 * 100.,
                                (i as f64 * 0.3).sin() * 20.,
                                -i as f64 * 300.,
                            )
                    })
                    .collect();
                for powered in [false, true] {
                    let tail = powered.then(|| *trail.last().unwrap() + DVec3::new(10., 0., -30.));
                    let mut all = trail.clone();
                    all.extend(tail);
                    let nozzle = *all.last().unwrap();
                    let distance = smoke_distances(&all, nozzle);
                    let (mut expected_pos, mut expected_col, mut expected_idx) =
                        (Vec::new(), Vec::new(), Vec::new());
                    for (i, pair) in all.windows(2).enumerate() {
                        let tangent = (pair[1] - pair[0]).normalize_or_zero();
                        let up = pair[0].normalize();
                        let right = tangent.cross(up).normalize_or_zero();
                        for axis in [up, right] {
                            let n = expected_pos.len() as u32;
                            for (end, point) in pair.iter().enumerate() {
                                let fraction = (i + end) as f32 / (all.len() - 1) as f32;
                                let width = (0.12 + 1.7 * (1. - fraction))
                                    * (1. + 0.10 * ((i + end) as f32 * 1.7).sin());
                                let alpha = (fraction * 4.).min(1.)
                                    * 0.75
                                    * smoke_clearance(distance[i + end]);
                                let center = world::local(*point, &f);
                                for (across, feather) in
                                    [(-1., 0.), (-0.5, 0.45), (0., 1.), (0.5, 0.45), (1., 0.)]
                                {
                                    expected_pos.push(
                                        (center + axis.as_vec3() * width * across).to_array(),
                                    );
                                    expected_col.push([1., 1., 1., alpha * feather]);
                                }
                            }
                            for j in 0..4 {
                                let k = n + j;
                                expected_idx.extend([k, k + 1, k + 5, k + 1, k + 6, k + 5]);
                            }
                        }
                    }
                    let mut asset = mesh(vec![Vec3::ZERO, Vec3::X, Vec3::Y], vec![0, 1, 2]);
                    let basis = world::frame(&f).conjugate();
                    let origin = world::render_origin(&f);
                    let bounds = write_smoke_mesh(&mut asset, &trail, tail, nozzle, basis, origin);
                    let Some(VertexAttributeValues::Float32x3(actual)) =
                        asset.attribute(Mesh::ATTRIBUTE_POSITION)
                    else {
                        panic!()
                    };
                    assert_eq!(*actual, expected_pos);
                    let pointer = actual.as_ptr();
                    for point in actual {
                        assert!(
                            (bevy::math::Vec3A::from(*point) - bounds.center)
                                .abs()
                                .cmple(bounds.half_extents)
                                .all()
                        );
                    }
                    let Some(VertexAttributeValues::Float32x4(actual)) =
                        asset.attribute(Mesh::ATTRIBUTE_COLOR)
                    else {
                        panic!()
                    };
                    assert_eq!(*actual, expected_col);
                    let Some(Indices::U32(actual)) = asset.indices() else {
                        panic!()
                    };
                    assert_eq!(*actual, expected_idx);
                    trail[0].x += 0.01;
                    write_smoke_mesh(&mut asset, &trail, tail, nozzle, basis, origin);
                    let Some(VertexAttributeValues::Float32x3(actual)) =
                        asset.attribute(Mesh::ATTRIBUTE_POSITION)
                    else {
                        panic!()
                    };
                    assert_eq!(
                        actual.as_ptr(),
                        pointer,
                        "steady-size trail reallocated positions"
                    );
                }
            }
        }
    }
    #[test]
    fn player_missile_destroys_enemy_and_keeps_hit_snapshot() {
        let f = flight();
        let mut c = Combat::default();
        c.start_air(&f, 1, 6);
        c.targets[0].damage.rng = 3;
        let t = &c.targets[0];
        let position = t.position + t.orientation * DVec3::new(-6.0, 0.0, 1.8);
        let velocity = t.velocity;
        let mut gnc = guidance::Gnc::launch(velocity, guidance::target(t), 1);
        gnc.active = true;
        c.missiles.push(Missile {
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
        c.advance(&f, 0.01);
        assert_eq!(c.strikes, 1);
        assert_eq!(c.hits, 1);
        assert!(!c.targets[0].alive);
        let report = c.recent_hit.as_ref().unwrap();
        assert_eq!(report.index, 0);
        assert!(report.destroyed);
        assert_eq!(report.integrity, 0.);
        c.advance(&f, 0.01);
        assert_eq!(c.hits, 1);
    }
    #[test]
    fn missile_camera_cycles_both_sides_and_restores_original_view() {
        let f = flight();
        let mut combat = Combat::default();
        combat.start(&f, 1);
        combat.lock = 1.0;
        assert!(combat.fire(&f));
        combat.clock += 1.0;
        combat.lock = 1.0;
        assert!(combat.fire(&f));
        let mut enemy = Combat::default();
        enemy.start_air(&f, 1, 6);
        combat.fight = enemy.fight;
        combat
            .fight
            .as_mut()
            .unwrap()
            .enemy_missiles
            .push(combat.missiles[0].clone());
        let view = ViewState {
            index: 3,
            yaw: 0.8,
            pitch: -0.2,
            zoom: 0.7,
            ..default()
        };
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(f)
            .insert_resource(combat)
            .insert_resource(view)
            .insert_resource(crate::warp::Warp::default())
            .add_systems(Update, camera_controls);
        for expected in [0, 1, MISSILE_COUNT, 0] {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::Digit6);
            app.update();
            let view = app.world().resource::<ViewState>();
            assert_eq!(view.missile_follow.unwrap().missile, expected);
            assert_eq!(view.index, 1);
            assert_eq!(app.world().resource::<Combat>().fired, 2);
        }
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        keys.press(KeyCode::Digit7);
        app.update();
        let view = app.world().resource::<ViewState>();
        assert!(view.missile_follow.is_none());
        assert_eq!(
            (view.index, view.yaw, view.pitch, view.zoom),
            (3, 0.8, -0.2, 0.7)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ViewState>()
            .follow_missile(0);
        app.world_mut().resource_mut::<Combat>().missiles[0].alive = false;
        app.update();
        assert_eq!(
            app.world()
                .resource::<ViewState>()
                .missile_follow
                .unwrap()
                .missile,
            1
        );
        {
            let mut combat = app.world_mut().resource_mut::<Combat>();
            combat.missiles[1].alive = false;
            combat.fight.as_mut().unwrap().enemy_missiles[0].alive = false;
        }
        app.update();
        assert!(app.world().resource::<ViewState>().missile_follow.is_none());
        assert_eq!(app.world().resource::<ViewState>().index, 3);
    }
    #[test]
    fn two_cycles_living_aircraft_and_seven_restores_without_changing_flight() {
        let f = flight();
        let original_position = f.position;
        let mut combat = Combat::default();
        combat.start(&f, 1);
        combat.targets[1].alive = false;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(f)
            .insert_resource(combat)
            .insert_resource(ViewState {
                index: 3,
                yaw: 0.8,
                pitch: -0.2,
                zoom: 0.7,
                ..default()
            })
            .insert_resource(crate::warp::Warp::default())
            .add_systems(Update, camera_controls);
        for expected in [0, 2, 0] {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::Digit2);
            app.update();
            assert_eq!(
                app.world().resource::<ViewState>().aircraft_follow,
                Some(expected)
            );
            assert_eq!(app.world().resource::<Flight>().position, original_position);
        }
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        keys.press(KeyCode::Digit7);
        app.update();
        let v = app.world().resource::<ViewState>();
        assert!(!v.following());
        assert_eq!((v.index, v.yaw, v.pitch, v.zoom), (3, 0.8, -0.2, 0.7));
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        keys.press(KeyCode::AltLeft);
        keys.press(KeyCode::Digit2);
        app.update();
        assert!(!app.world().resource::<ViewState>().following());
        app.world_mut()
            .resource_mut::<ViewState>()
            .follow_aircraft(0);
        app.world_mut().resource_mut::<Combat>().targets[0].alive = false;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.update();
        assert_eq!(app.world().resource::<ViewState>().aircraft_follow, Some(2));
        app.world_mut().resource_mut::<Combat>().targets[2].alive = false;
        app.update();
        assert!(!app.world().resource::<ViewState>().following());
    }
    #[test]
    fn keyboard_shortcuts_dispense_individual_countermeasures() {
        let f = flight();
        let mut combat = Combat::default();
        combat.start_air(&f, 1, 6);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(f)
            .insert_resource(combat)
            .insert_resource(ViewState {
                generation: 1,
                scenario: 6,
                ..default()
            })
            .insert_resource(crate::warp::Warp::default())
            .add_systems(Update, controls);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyV);
        app.update();
        let fight = app.world().resource::<Combat>().fight.as_ref().unwrap();
        assert_eq!((fight.flares, fight.chaff), (23, 23));
        assert_eq!(fight.decoys[0].kind, Weapon::Heat);
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        keys.press(KeyCode::Backspace);
        app.update();
        let fight = app.world().resource::<Combat>().fight.as_ref().unwrap();
        assert_eq!((fight.flares, fight.chaff), (23, 23));
        assert_eq!(fight.decoys[1].kind, Weapon::Radar);
    }
    #[test]
    fn no_launch_without_lock_and_inventory_limits_launches() {
        let f = flight();
        let mut c = Combat::default();
        c.start(&f, 1);
        assert!(!c.fire(&f));
        assert_eq!(c.ammo, [2, 2]);
        for kind in [Weapon::Heat, Weapon::Radar] {
            c.weapon = kind;
            c.lock = kind.lock_time();
            assert!(c.fire(&f));
            c.clock += 1.0;
            assert!(c.fire(&f));
            c.clock += 1.0;
            assert!(!c.fire(&f));
        }
        assert_eq!(c.ammo, [0, 0]);
        assert_eq!(c.missiles.len(), 4);
    }
    #[test]
    fn each_weapon_intercepts_moving_target_with_fixed_substeps() {
        for kind in [Weapon::Heat, Weapon::Radar] {
            let f = flight();
            let mut c = Combat::default();
            c.start(&f, 1);
            c.weapon = kind;
            for _ in 0..180 {
                c.advance(&f, 1.0 / 120.0);
            }
            assert!(c.fire(&f));
            for _ in 0..3600 {
                c.advance(&f, 1.0 / 120.0);
                if c.strikes > 0 {
                    break;
                }
            }
            assert_eq!(c.strikes, 1, "{:?}", kind);
            assert!(c.targets[0].health < 1.0, "{:?}", kind);
            assert!(
                c.missiles
                    .iter()
                    .all(|m| m.position.is_finite() && m.velocity.is_finite())
            );
        }
    }
    #[test]
    fn scenario_acquires_and_intercepts_while_player_physics_runs() {
        let mut f = flight();
        let mut e = Environment::default();
        let mut c = Combat::default();
        c.start(&f, 1);
        for _ in 0..144 {
            e.advance(1.0 / 120.0);
            simulate(&mut f, &e, 1.0 / 120.0);
            c.advance(&f, 1.0 / 120.0);
        }
        assert!(
            c.fire(&f),
            "{} / pitch {:.1}",
            c.message,
            f.pitch().to_degrees()
        );
        for _ in 0..3600 {
            e.advance(1.0 / 120.0);
            simulate(&mut f, &e, 1.0 / 120.0);
            c.advance(&f, 1.0 / 120.0);
            if c.strikes > 0 {
                break;
            }
        }
        assert_eq!(c.strikes, 1);
        assert!(c.targets[0].health < 1.0);
        assert!(!f.crashed);
    }
    #[test]
    fn integrated_step_shares_wall_time_and_pause_with_targets_and_missiles() {
        let f = flight();
        let mut c = Combat::default();
        c.start(&f, 1);
        c.lock = 1.0;
        assert!(c.fire(&f));
        let mut warp = crate::warp::Warp::default();
        warp.timestep = 0.005;
        let mut app = App::new();
        app.init_resource::<Time<bevy::time::Real>>()
            .insert_resource(f)
            .insert_resource(c)
            .insert_resource(Environment::default())
            .insert_resource(warp)
            .insert_resource(crate::SimClock::default())
            .add_systems(Update, crate::step);
        for _ in 0..5 {
            app.world_mut().resource_mut::<Time<bevy::time::Real>>().advance_by(std::time::Duration::from_millis(10));
            app.update();
        }
        assert!((app.world().resource::<Combat>().missiles[0].age - 0.05).abs() < 1e-9);
        let position = app.world().resource::<Combat>().targets[0].position;
        app.world_mut().resource_mut::<Flight>().paused = true;
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<Combat>().targets[0].position,
            position
        );
        assert!((app.world().resource::<Combat>().missiles[0].age - 0.05).abs() < 1e-9);
    }
    #[test]
    fn heat_and_radar_allow_head_on_acquisition() {
        let f = flight();
        let mut c = Combat::default();
        c.start(&f, 1);
        c.targets[0].velocity = -f.forward() * 170.0;
        c.targets[0].orientation = facing(-f.forward(), f.up());
        assert!(c.cue(&f).is_ok());
        c.weapon = Weapon::Radar;
        assert!(c.cue(&f).is_ok());
    }
    #[test]
    fn relative_sweep_catches_high_speed_crossing() {
        assert!(swept_distance(DVec3::new(-100.0, 5.0, 0.0), DVec3::new(100.0, 5.0, 0.0)) < 6.0);
        assert!(swept_distance(DVec3::new(-100.0, 30.0, 0.0), DVec3::new(100.0, 30.0, 0.0)) > 18.0);
    }
    #[test]
    fn routes_use_shared_dynamics_and_failed_seeker_goes_ballistic() {
        let f = flight();
        let mut c = Combat::default();
        c.start(&f, 1);
        c.lock = 1.0;
        assert!(c.fire(&f));
        c.targets[0].alive = false;
        for _ in 0..240 {
            c.advance(&f, 1.0 / 120.0);
        }
        assert!(!c.missiles[0].guided);
        assert!(c.missiles[0].alive);
        for t in c.targets.iter().filter(|t| t.alive) {
            assert_eq!(t.position, t.dynamics.position);
            assert_eq!(t.velocity, t.dynamics.velocity);
            assert!((t.position.length() - RADIUS - t.altitude).abs() < 50.0);
            assert!(t.brain.physics_steps >= 240);
            assert!(t.fuel < 2400.);
        }
    }
    #[test]
    fn scenario_reset_clears_weapons_and_keyboard_controls_select_and_fire() {
        let mut app = App::new();
        let f = flight();
        let mut combat = Combat::default();
        combat.start(&f, 1);
        let view = ViewState {
            generation: 1,
            scenario: 4,
            ..default()
        };
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(f)
            .insert_resource(combat)
            .insert_resource(view)
            .insert_resource(crate::warp::Warp::default())
            .add_systems(Update, controls);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.update();
        assert_eq!(app.world().resource::<Combat>().selected, 1);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyO);
        app.update();
        assert_eq!(app.world().resource::<Combat>().weapon, Weapon::Radar);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut().resource_mut::<Combat>().lock = Weapon::Radar.lock_time();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.update();
        assert_eq!(app.world().resource::<Combat>().fired, 1);
        {
            let mut view = app.world_mut().resource_mut::<ViewState>();
            view.generation = 2;
            view.scenario = 3;
        }
        app.update();
        let combat = app.world().resource::<Combat>();
        assert!(!combat.active);
        assert!(combat.targets.is_empty());
        assert!(combat.missiles.is_empty());
    }
}

#[cfg(test)]
mod side_bay_tests;
