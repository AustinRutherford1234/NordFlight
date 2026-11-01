use super::*;
fn closest(a: DVec3, b: DVec3) -> f64 {
    let d = b - a;
    (a + d * (-a.dot(d) / d.length_squared().max(1e-12)).clamp(0., 1.)).length()
}
impl State {
    fn contact(&self, id: TargetId, c: &Combat, f: &Flight) -> Option<(DVec3, DVec3)> {
        match id {
            TargetId::Aircraft(id) => {
                let r = self.raiders.iter().find(|r| r.id == id)?;
                let t = c.targets.get(r.slot).filter(|t| t.alive)?;
                Some((t.position, t.velocity))
            }
            TargetId::Missile(id) => {
                let m = self.rounds.iter().find(|m| m.id == id && m.alive)?;
                Some((m.position, m.velocity))
            }
            TargetId::Ship(s) => (!self.ships[s].sunk).then(|| {
                (
                    carrier::ship_world(f, s, DVec3::Y * 5., f.elapsed),
                    carrier::ship_velocity(f, s, carrier::ship_origin(f, s, f.elapsed), f.elapsed),
                )
            }),
            TargetId::Player => {
                (!f.crashed && !f.ejection.active).then_some((f.position, f.velocity))
            }
        }
    }
    pub(super) fn round(
        &mut self,
        source: TargetId,
        target: TargetId,
        kind: Kind,
        p: DVec3,
        v: DVec3,
    ) -> bool {
        if self.rounds.iter().filter(|m| m.alive).count() >= self.weapon_budget {
            return false;
        }
        let id = self.id();
        let round = Round {
            sam: Default::default(),
            id,
            source,
            target,
            kind,
            position: p,
            previous: p,
            velocity: v,
            age: 0.,
            alive: true,
            support_lost: 0.,
            next_sensor: 0.,
            visual:render_round(p,v),
        };
        if let Some(slot) = self.rounds.iter_mut().find(|m| !m.alive) {
            *slot = round;
            true
        } else if self.rounds.len() < WEAPON_LIMIT {
            self.rounds.push(round);
            true
        } else {
            false
        }
    }
    fn flash(&mut self, p: DVec3, water: bool) {
        if self.flashes.len() == 32 {
            self.flashes.remove(0);
        }
        self.flashes.push(Flash {
            position: p,
            age: 0.,
            water,
        });
    }
    fn intercept(
        &mut self,
        target: TargetId,
        c: &mut Combat,
        f: &mut Flight,
        point: DVec3,
        energy: f64,
    ) {
        match target {
            TargetId::Missile(id) => {
                if let Some(m) = self.rounds.iter_mut().find(|m| m.id == id && m.alive) {
                    m.alive = false;
                    self.interceptions += 1;
                }
            }
            TargetId::Aircraft(id) => {
                if let Some(r) = self.raiders.iter().find(|r| r.id == id) {
                    let t = &mut c.targets[r.slot];
                    if t.alive {
                        t.health -= energy;
                        if t.health <= 0. {
                            t.alive = false;
                            t.wreck_age = 0.;
                            t.damage.fire = 1.;
                            self.ship_kills += 1;
                        }
                    }
                }
            }
            TargetId::Ship(s) => {
                let p = carrier::ship_relative(f, s, point, f.elapsed);
                self.ships[s].hit(p, energy);
            }
            TargetId::Player => {
                if !f.crashed {
                    f.integrity = (f.integrity - energy * 0.5).max(0.);
                    f.damage.avionics = (f.damage.avionics - energy * 0.3).max(0.);
                    if f.integrity <= 0. {
                        let up = f.up();
                        crate::collision::destroy(f, "HOSTILE MISSILE HIT", up);
                    }
                }
            }
        }
        crate::sound::emit(
            &mut c.audio_events,
            crate::sound::Kind::Blast,
            point,
            DVec3::ZERO,
            false,
        );
        self.flash(point, false);
    }
    fn obstructed(&self, f: &Flight, _ship: usize, from: DVec3, to: DVec3) -> bool {
        let direction = (to - from).normalize_or_zero();
        carrier::collider::sweep(f, from + direction * 4., to, 0., 0.2)
            .is_some_and(|h| h.fraction < 0.99)
    }
    fn engage(&mut self, c: &Combat, f: &Flight) {
        if self.clock < self.next_engage {
            return;
        }
        self.next_engage = self.clock + 0.25;
        // Fixed stack storage: missile threats sorted by time-to-fleet, then aircraft.
        let mut threats = [(TargetId::Player, f64::INFINITY); 108];
        let mut count = 0;
        for m in self
            .rounds
            .iter()
            .filter(|m| m.alive && m.kind == Kind::AntiShip)
        {
            threats[count] = (
                TargetId::Missile(m.id),
                m.position.distance(carrier::ship_origin(f, 0, f.elapsed))
                    / m.velocity.length().max(1.),
            );
            count += 1;
        }
        for r in &self.raiders {
            if self
                .tracks
                .iter()
                .any(|t| t.id == r.id && t.valid(self.clock))
                && c.targets[r.slot].alive
            {
                threats[count] = (
                    TargetId::Aircraft(r.id),
                    1000.
                        + c.targets[r.slot]
                            .position
                            .distance(carrier::ship_origin(f, 0, f.elapsed))
                            / 250.,
                );
                count += 1;
            }
        }
        threats[..count].sort_unstable_by(|a, b| a.1.total_cmp(&b.1));
        for ship in 0..3 {
            if self.ships[ship].power < 0.2
                || self.ships[ship].launcher < 0.25
                || self.ships[ship].sunk
            {
                continue;
            }
            for &(target, _) in &threats[..count] {
                let Some((p, _)) = self.contact(target, c, f) else {
                    continue;
                };
                let mount = carrier::ship_world(f, ship, launch_mount(ship), f.elapsed);
                let range = p.distance(mount);
                if (matches!(target, TargetId::Aircraft(_)) && range > 22000.)
                    || range < 500.
                    || !self.detect(
                        f,
                        ship,
                        p,
                        if matches!(target, TargetId::Missile(_)) {
                            0.1
                        } else {
                            3.
                        },
                    )
                    || self.obstructed(f, ship, mount, p)
                {
                    continue;
                }
                let committed = self
                    .rounds
                    .iter()
                    .filter(|m| {
                        m.alive
                            && m.target == target
                            && !matches!(m.kind, Kind::AntiShip | Kind::AirToAir)
                    })
                    .count();
                if committed >= 2 {
                    continue;
                }
                if self.clock < self.ships[ship].next_launch {
                    break;
                }
                let s = &self.ships[ship];
                let kind = if range < 18000. && s.essm > 0 {
                    Kind::Essm
                } else if range < 55000. && s.standard > 0 {
                    Kind::Standard
                } else {
                    continue;
                };
                if self.round(
                    TargetId::Ship(ship),
                    target,
                    kind,
                    mount,
                    carrier::ship_rotation(f, ship, f.elapsed) * DVec3::Y * 35.
                        + carrier::velocity(f.elapsed),
                ) {
                    let s = &mut self.ships[ship];
                    if kind == Kind::Standard {
                        s.standard -= 1;
                    } else {
                        s.essm -= 1;
                    }
                    s.next_launch = self.clock + if ship == 0 { 3. } else { 2. };
                }
                break;
            }
        }
    }
    pub(super) fn hostile_launches(&mut self, c: &Combat, f: &Flight) {
        for i in 0..self.raiders.len() {
            let r = &self.raiders[i];
            let t = &c.targets[r.slot];
            if !t.alive || self.clock < r.next_shot {
                continue;
            }
            if r.role==Role::Bomber&&(r.bay<0.98 || t.dynamics.mach>0.95){continue;}
            let range = t.position.distance(carrier::ship_origin(f, 0, f.elapsed));
            let player = f.position - t.position;
            let engage_player = r.aam > 0
                && r.role == Role::Fighter
                && !f.crashed
                && !f.ejection.active
                && t.brain.memory.is_some_and(|(_,_,at)|t.brain.elapsed-at<3.)
                && player.length() < 14000.
                && (t.orientation * -DVec3::Z).dot(player.normalize_or_zero()) > 0.75;
            let ship = if r.role==Role::Bomber{0}else{(0..3)
                .filter(|&s| !self.ships[s].sunk)
                .min_by(|&a, &b| {
                    carrier::ship_origin(f, a, f.elapsed)
                        .distance_squared(t.position)
                        .total_cmp(
                            &carrier::ship_origin(f, b, f.elapsed).distance_squared(t.position),
                        )
                })
                .unwrap_or(0)};
            let attack = r.stores > 0 && !r.withdrew && range < 26000.
                && (t.orientation*-DVec3::Z).dot((carrier::ship_origin(f,ship,f.elapsed)-t.position).normalize_or_zero())>0.5;
            let source = TargetId::Aircraft(r.id);
            if engage_player || attack {
                let target = if engage_player {
                    TargetId::Player
                } else {
                    TargetId::Ship(ship)
                };
                let kind = if engage_player {
                    Kind::AirToAir
                } else {
                    Kind::AntiShip
                };
                let p = if r.role==Role::Bomber{t.position+t.orientation*super::bomber::release_station(r.stores)}else{t.position-t.position.normalize()*2.};
                if self.round(source, target, kind, p, t.velocity) {
                    if let Some((position,velocity))=self.contact(target,c,f) {
                        if let Some(m)=self.rounds.iter_mut().find(|m|m.id==self.next_id-1) {tracking::seed(m,position,velocity);}
                    }
                    let r = &mut self.raiders[i];
                    if engage_player {
                        r.aam -= 1;
                    }
                    r.next_shot = self.clock + if engage_player { 25. } else { 5. };
                    if attack && !engage_player {
                        r.stores -= 1;
                        r.withdrew = r.stores == 0;
                    }
                }
            }
        }
    }
    fn ciws(&mut self, c: &mut Combat, f: &mut Flight, dt: f64) {
        for ship in 0..3 {
            let s = &self.ships[ship];
            if s.power < 0.2 || s.sunk || s.gun < 30 {
                continue;
            }
            let mount = carrier::ship_world(f, ship, gun_mount(ship), f.elapsed);
            let candidate = self
                .rounds
                .iter()
                .filter(|m| {
                    m.alive && m.kind == Kind::AntiShip && m.position.distance(mount) < 2500.
                })
                .min_by(|a, b| {
                    a.position
                        .distance_squared(mount)
                        .total_cmp(&b.position.distance_squared(mount))
                })
                .map(|m| TargetId::Missile(m.id))
                .or_else(|| {
                    self.raiders
                        .iter()
                        .filter(|r| {
                            c.targets[r.slot].alive
                                && self.track_for(r.slot).is_some()
                                && c.targets[r.slot].position.distance(mount) < 1800.
                        })
                        .map(|r| TargetId::Aircraft(r.id))
                        .next()
                });
            let Some(target) = candidate else {
                continue;
            };
            let Some((p, v)) = self.contact(target, c, f) else {
                continue;
            };
            if !self.detect(f, ship, p, 0.1) || self.obstructed(f, ship, mount, p) {
                continue;
            }
            let relative_v = v - carrier::ship_velocity(f, ship, mount, f.elapsed);
            let r = p - mount;
            let a = relative_v.length_squared() - 1100f64.powi(2);
            let b = 2. * r.dot(relative_v);
            let disc = (b * b - 4. * a * r.length_squared()).max(0.).sqrt();
            let flight = if a.abs() > 1e-8 {
                ((-b - disc) / (2. * a)).max(0.)
            } else {
                r.length() / 1100.
            };
            let local = carrier::ship_rotation(f, ship, f.elapsed).conjugate()
                * (r + relative_v * flight + mount.normalize() * (4.905 * flight * flight));
            let yaw = local.x.atan2(-local.z);
            let pitch = local.y.atan2(local.x.hypot(local.z));
            if !(-0.15..1.45).contains(&pitch) {
                continue;
            }
            let s = &mut self.ships[ship];
            let dy = (yaw - s.gun_yaw + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
                - std::f64::consts::PI;
            let rate = 2.0 * dt;
            s.gun_yaw += dy.clamp(-rate, rate);
            s.gun_pitch += (pitch - s.gun_pitch).clamp(-rate, rate);
            if dy.abs() > 0.025 || (pitch - s.gun_pitch).abs() > 0.025 || self.clock < s.next_burst
            {
                continue;
            }
            let direction = carrier::ship_rotation(f, ship, f.elapsed)
                * (DVec3::new(
                    s.gun_yaw.sin() * s.gun_pitch.cos(),
                    s.gun_pitch.sin(),
                    -s.gun_yaw.cos() * s.gun_pitch.cos(),
                ));
            let burst = Burst {
                ship,
                target,
                position: mount,
                previous: mount,
                velocity: direction * 1100. + carrier::ship_velocity(f, ship, mount, f.elapsed),
                age: 0.,
                rounds: 30,
                alive: true,
            };
            if let Some(b) = self.bursts.iter_mut().find(|b| !b.alive) {
                *b = burst;
            } else if self.bursts.len() < 96 {
                self.bursts.push(burst);
            } else {
                continue;
            }
            s.gun -= 30;
            s.next_burst = self.clock + 0.5;
        }
    }
    fn countermeasures(&mut self, c:&mut Combat, f:&Flight) {
        if self.clock<self.next_countermeasures {return;}
        self.next_countermeasures=self.clock+0.2;
        let Some(fight)=c.fight.as_mut() else{return;};
        for r in &mut self.raiders {
            let t=&c.targets[r.slot];
            if !t.alive || r.chaff==0 || self.clock<r.next_chaff || fight.decoys.len()>126 {continue;}
            let threat=self.rounds.iter().any(|m|m.alive && m.target==TargetId::Aircraft(r.id)
                && m.position.distance(t.position)<6500. && guidance::terrain_visible(t.position,m.position))
                || c.missiles.iter().any(|m|m.alive && m.target==r.slot && m.position.distance(t.position)<5000.
                    && guidance::terrain_visible(t.position,m.position));
            if !threat {continue;}
            r.chaff-=1;r.next_chaff=self.clock+6.;
            for kind in [Weapon::Radar,Weapon::Heat] {
                let id=fight.next_decoy;fight.next_decoy+=1;
                fight.decoys.push(dogfight::Decoy{owner:Some(r.slot),id,kind,
                    position:t.position-t.orientation*DVec3::Z*3.,
                    velocity:t.velocity+t.orientation*DVec3::new(12.,-8.,30.),age:0.});
            }
        }
        for ship in 0..3 {
            if self.ships[ship].sunk || self.ships[ship].power<0.2 || self.ship_chaff[ship]==0
                || self.clock<self.next_ship_chaff[ship] || fight.decoys.len()>=128 {continue;}
            let position=carrier::ship_world(f,ship,DVec3::new(-10.,18.,25.),f.elapsed);
            let incoming=self.rounds.iter().any(|m|m.alive && m.target==TargetId::Ship(ship)
                && m.position.distance(position)<6000.
                && self.missile_tracks.iter().any(|t|t.id==m.id && t.valid(self.clock)));
            if !incoming {continue;}
            self.ship_chaff[ship]-=1;self.next_ship_chaff[ship]=self.clock+8.;
            let id=fight.next_decoy;fight.next_decoy+=1;
            fight.decoys.push(dogfight::Decoy{owner:None,id,kind:Weapon::Radar,position,
                velocity:carrier::ship_velocity(f,ship,position,f.elapsed)+carrier::ship_rotation(f,ship,f.elapsed)*DVec3::new(-15.,18.,0.),age:0.});
        }
    }
    pub fn weapons(&mut self, c: &mut Combat, f: &mut Flight, dt: f64) {
        if !self.ended {
            self.countermeasures(c, f);
            self.hostile_launches(c, f);
            self.engage(c, f);
            self.ciws(c, f, dt);
        }
        // Snapshot avoids mutable target aliases and iteration-order dependent steering.
        let mut endpoints = [(0u64, DVec3::ZERO, DVec3::ZERO, false); WEAPON_LIMIT];
        for (i, m) in self.rounds.iter().enumerate() {
            endpoints[i] = (m.id, m.position, m.velocity, m.alive);
        }
        for i in 0..self.rounds.len() {
            if !self.rounds[i].alive {
                continue;
            }
            let target = self.rounds[i].target;
            let contact = if let TargetId::Missile(id) = target {
                endpoints
                    .iter()
                    .find(|m| m.0 == id && m.3)
                    .map(|m| (m.1, m.2))
            } else {
                self.contact(target, c, f)
            };
            let target_contact=contact.map(|(position,velocity)|guidance::Contact {
                position,velocity,forward:velocity.normalize_or_zero(),throttle:1.,engine:true,
                alive:true,decoy:false,strength:match target {TargetId::Ship(_)=>100.,TargetId::Missile(_)=>0.15,_=>1.},
            });
            let support = match self.rounds[i].source {
                TargetId::Ship(s) if self.ships[s].radar*self.ships[s].power>0.2 && !self.ships[s].sunk => {
                    let tracks=if matches!(target,TargetId::Missile(_)){&self.missile_tracks}else{&self.tracks};
                    let id=match target{TargetId::Aircraft(id)|TargetId::Missile(id)=>Some(id),_=>None};
                    tracks.iter().find(|t|Some(t.id)==id && t.valid(self.clock) && self.clock-t.seen<3.)
                        .map(|t|(t.position+t.velocity*(self.clock-t.seen),t.velocity))
                }
                TargetId::Aircraft(id) if target==TargetId::Player => self.raiders.iter().find(|r|r.id==id)
                    .and_then(|r|c.targets.get(r.slot)).filter(|t|t.alive && t.damage.radar>0.25
                        && (t.orientation*-DVec3::Z).dot((f.position-t.position).normalize_or_zero())>65f64.to_radians().cos())
                    .and_then(|t|t.brain.memory.filter(|(_,_,at)|t.brain.elapsed-at<3.)
                        .map(|(p,v,at)|(p+v*(t.brain.elapsed-at),v))),
                _=>None,
            };
            let m = &mut self.rounds[i];
            m.previous = m.position;
            m.age += dt;
            let (max_speed, accel, turn, burn, life) = match m.kind {
                Kind::Standard => (1050., 110., 30., 14., 100.),
                Kind::Essm => (950., 160., 40., 9., 50.),
                Kind::AntiShip => (290., 45., 6., 120., 180.),
                Kind::AirToAir => (850., 150., 25., 7., 45.),
            };
            if support.is_none() {m.support_lost+=dt;}else{m.support_lost=0.;}
            // A seeded launch coordinate is available to the anti-ship INS. Other
            // weapons need an actual datalink or seeker measurement to initialize.
            if m.kind==Kind::AntiShip && !m.visual.gnc.initialized {
                if let Some((p,v))=contact {tracking::seed(m,p,v);}
            }
            let tracked=tracking::update(m,target_contact,support,
                c.fight.as_ref().map_or(&[],|fight|fight.decoys.as_slice()),c.sun,dt);
            if matches!(m.kind,Kind::Standard|Kind::Essm) {
                sam::advance(m,tracked,dt);
            } else {
            let (estimate,estimated_velocity)=tracked.unwrap_or((m.position+m.velocity,DVec3::ZERO));
            let delta = estimate - m.position;
            let intercept = (delta.length() / m.velocity.length().max(200.)).min(12.);
            let mut goal = estimate + estimated_velocity * intercept;
            if m.kind == Kind::AntiShip {
                let height = if delta.length() > 5000. { 18. } else { 5. };
                let terrain = terrain_height(goal.x, goal.z).max(0.);
                goal = goal.normalize() * (RADIUS + height + terrain);
            }
            if matches!(m.source, TargetId::Ship(_)) && m.age < 0.6 {
                goal = m.position + m.position.normalize() * 500.;
            }
            let speed = m.velocity.length().max(1.);
            let direction = m.velocity / speed;
            let desired = (goal - m.position).normalize_or(direction);
            let lateral = (desired - direction * desired.dot(direction)) * speed * 3.;
            let up = m.position.normalize();
            let gravity = up * 9.81;
            let compensation = gravity - direction * gravity.dot(direction);
            let guidance = if tracked.is_none() {
                DVec3::ZERO
            } else {
                (lateral + compensation).clamp_length_max(turn * 9.81)
            };
            m.visual.gnc.commanded_acceleration=guidance;
            m.visual.gnc.actual_g=guidance.length()/9.81;
            let thrust = if m.age < burn && speed < max_speed {
                accel
            } else {
                0.
            };
            let drag = 0.000025 * speed * speed;
            m.velocity +=
                (direction * (thrust - drag) + guidance - m.position.normalize() * 9.81) * dt;
            m.position += m.velocity * dt;
            }
            if !m.alive || !m.position.is_finite()
                || m.age > life
                || m.position.length() < RADIUS + terrain_height(m.position.x, m.position.z).max(0.)
            {
                m.alive = false;
                continue;
            }
            // Ship collision uses the actual imported moving hull; exclude the launch hull briefly.
            let hull = carrier::collider::sweep(f, m.previous, m.position, dt, 0.25)
                .filter(|h| !(m.age < 0.8 && m.source == TargetId::Ship(h.ship)));
            let fuse = if matches!(m.kind, Kind::AntiShip) {
                4.
            } else {
                14.
            };
            let hit = contact.is_some_and(|(target_p,target_v)|closest(
                m.previous - (target_p - target_v * dt),m.position - target_p,
            ) < fuse);
            let event = if let Some(h) = hull {
                Some((TargetId::Ship(h.ship), h.point, 1.2))
            } else if hit && m.age > 0.5 {
                Some((
                    target,
                    m.position,
                    if m.kind == Kind::AntiShip { 1.2 } else { 2.5 },
                ))
            } else {
                None
            };
            if let Some((target, p, e)) = event {
                self.rounds[i].alive = false;
                self.intercept(target, c, f, p, e);
            }
        }
        // Rendering adapter shares the AIM-120 smoke path without running a second
        // weapon solver. Retained samples are bounded and cleared on slot reuse.
        for i in 0..self.rounds.len() {
            let m=&mut self.rounds[i];let sam_g=if matches!(m.kind,Kind::Standard|Kind::Essm){sam::available_g(m)}else{0.};let v=&mut m.visual;
            v.gnc.exhaust_offset=super::ordnance::dimensions(m.kind).0 as f64*0.49;v.gnc.available_g=if matches!(m.kind,Kind::Standard|Kind::Essm){sam_g}else{match m.kind{Kind::Standard=>30.,Kind::Essm=>40.,Kind::AntiShip=>6.,Kind::AirToAir=>25.}};
            v.position=m.position;v.velocity=m.velocity;v.age=m.age;v.alive=m.alive;v.gnc.body=m.velocity.normalize_or(DVec3::NEG_Z);v.gnc.attitude=facing(v.gnc.body,m.position.normalize());v.gnc.thrust=if m.age<m.kind.burn(){1.}else{0.};
            v.trail_clock+=dt;
            if v.trail_clock>=0.075 {v.trail_clock=v.trail_clock.rem_euclid(0.075);if v.gnc.thrust>0.{if v.trail.len()==64{v.trail.remove(0);}v.trail.push(m.position);}else if !v.trail.is_empty(){v.trail.remove(0);}}
        }
        for m in self.rounds.iter().filter(|m| m.age <= dt * 1.01) {
            crate::sound::emit(
                &mut c.audio_events,
                crate::sound::Kind::Launch,
                m.position,
                m.velocity,
                false,
            );
        }
        for b in self.bursts.iter().filter(|b| b.age == 0.) {
            crate::sound::emit(
                &mut c.audio_events,
                crate::sound::Kind::Cannon,
                b.position,
                b.velocity,
                false,
            );
        }
        for i in 0..self.bursts.len() {
            if !self.bursts[i].alive {
                continue;
            }
            let target = self.bursts[i].target;
            let contact = self.contact(target, c, f);
            let b = &mut self.bursts[i];
            b.previous = b.position;
            b.age += dt;
            b.velocity -= b.position.normalize() * 9.81 * dt;
            b.position += b.velocity * dt;
            if b.age > 3. || b.position.length() < RADIUS {
                b.alive = false;
                continue;
            }
            if carrier::collider::sweep(f, b.previous, b.position, dt, 0.02)
                .is_some_and(|h| b.age > 0.1 || h.ship != b.ship)
            {
                b.alive = false;
                continue;
            }
            if let Some((p, v)) = contact {
                if closest(b.previous - (p - v * dt), b.position - p)
                    < (1. + b.age * 1100. * 0.0025)
                {
                    let impact = b.position;
                    let energy = b.rounds as f64 * 0.025;
                    b.alive = false;
                    self.intercept(target, c, f, impact, energy);
                }
            }
        }
    }
}
// Initial attachment estimates in metres, ship-local; validate against native views.
pub fn launch_mount(ship: usize) -> DVec3 {
    if ship == 0 {
        DVec3::new(-31., 25., 105.)
    } else {
        DVec3::new(0., 7.8, -40.)
    }
}
pub fn gun_mount(ship: usize) -> DVec3 {
    if ship == 0 {
        DVec3::new(-35., 23.2, 30.)
    } else {
        DVec3::new(0., 18., 15.)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn swept_fuse_catches_fast_crossing() {
        assert!(closest(DVec3::new(-100., 2., 0.), DVec3::new(100., 2., 0.)) < 3.);
        assert!(closest(DVec3::new(-100., 20., 0.), DVec3::new(100., 20., 0.)) > 14.);
    }
}
#[cfg(test)]
mod engagement_tests {
    use super::*;
    #[test]
    fn standard_engagement_intercepts_a_moving_aircraft() {
        let mut f = flight(true);
        let mut c = Combat::default();
        start(&mut c, &f, 1, 1);
        let mut n = c.naval.take().unwrap();
        let id = n.raiders[0].id;
        n.raiders.truncate(1);
        for t in &mut c.targets {
            t.alive = false;
        }
        c.targets[0].alive = true;
        c.targets[0].health = 1.;
        let mut caught = false;
        for tick in 0..9600 {
            let time = tick as f64 / 120.;
            f.elapsed = time;
            n.clock = time;
            let p = carrier::world(DVec3::new(2500., 2200., -16000. + 180. * time), time);
            c.targets[0].position = p;
            c.targets[0].velocity = carrier::rotation(time) * DVec3::Z * 180.;
            n.tracks = vec![Track {
                id,
                position: p,
                velocity: c.targets[0].velocity,
                seen: time,
                hits: 5,
                role: Some(Role::Fighter),
            }];
            n.raiders[0].stores = 0;
            n.raiders[0].next_shot = f64::INFINITY;
            n.weapons(&mut c, &mut f, 1. / 120.);
            if !c.targets[0].alive {
                caught = true;
                break;
            }
        }
        assert!(caught, "SAM failed: {:?}", n.rounds);
        assert_eq!(n.ship_kills, 1);
        assert!(n.ships.iter().any(|s| s.standard < 24 || s.essm < 32));
    }
    #[test]
    fn antiship_round_hits_hull_with_defenses_disabled() {
        let mut f = flight(true);
        let mut c = Combat::default();
        let mut n = State::new(1, 3, false);
        n.next_wave = f64::INFINITY;
        for s in &mut n.ships {
            s.radar = 0.;
            s.gun = 0;
        }
        let p = carrier::world(DVec3::new(0., 18., -5000.), 0.);
        assert!(n.round(
            TargetId::Aircraft(1),
            TargetId::Ship(0),
            Kind::AntiShip,
            p,
            carrier::rotation(0.) * DVec3::Z * 280.
        ));
        for k in 0..6000 {
            f.elapsed = k as f64 / 120.;
            n.clock = f.elapsed;
            n.weapons(&mut c, &mut f, 1. / 120.);
            if !n.rounds[0].alive {
                break;
            }
        }
        assert!(
            n.ships[0].hull < 1.,
            "No physical hull strike: {:?}",
            n.rounds[0]
        );
    }
    #[test]
    fn ammunition_channels_and_support_are_bounded() {
        let mut n = State::new(1, 2, false);
        for i in 0..96 {
            assert!(n.round(
                TargetId::Ship(1),
                TargetId::Aircraft(i),
                Kind::Essm,
                DVec3::Y * RADIUS,
                DVec3::Y * 30.
            ));
        }
        assert!(!n.round(
            TargetId::Ship(1),
            TargetId::Aircraft(100),
            Kind::Essm,
            DVec3::Y * RADIUS,
            DVec3::Y * 30.
        ));
        n.rounds[0].alive = false;
        assert!(n.round(
            TargetId::Ship(1),
            TargetId::Aircraft(101),
            Kind::Essm,
            DVec3::Y * RADIUS,
            DVec3::Y * 30.
        ));
        assert_eq!(n.rounds.len(), 96);
    }
}
#[cfg(test)]
mod defense_tests {
    use super::*;
    #[test]
    fn ciws_intercepts_incoming_missile_using_ballistic_bursts() {
        let mut f = flight(true);
        let mut c = Combat::default();
        let mut n = State::new(1, 2, false);
        for s in &mut n.ships {
            s.standard = 0;
            s.essm = 0;
        }
        let p = carrier::ship_world(&f, 1, DVec3::new(0., 18., 3000.), 0.);
        n.round(
            TargetId::Aircraft(1),
            TargetId::Ship(1),
            Kind::AntiShip,
            p,
            carrier::rotation(0.) * DVec3::NEG_Z * 280.,
        );
        let initial = n.ships[1].hull;
        for i in 0..3000 {
            f.elapsed = i as f64 / 120.;
            n.clock = f.elapsed;
            n.weapons(&mut c, &mut f, 1. / 120.);
            if !n.rounds[0].alive {
                break;
            }
        }
        assert!(
            n.interceptions > 0,
            "CIWS missed; hull={} rounds={:?}",
            n.ships[1].hull,
            n.bursts
        );
        assert_eq!(n.ships[1].hull, initial);
        assert!(n.ships[1].gun < 3100);
    }
    #[test]
    fn ship_superstructure_blocks_gun_line() {
        let f = flight(true);
        let n = State::new(1, 2, false);
        let a = carrier::ship_world(&f, 1, DVec3::new(0., 12., 60.), 0.);
        let b = carrier::ship_world(&f, 1, DVec3::new(0., 12., -60.), 0.);
        assert!(n.obstructed(&f, 1, a, b));
    }
}

#[cfg(test)]mod raid_target_tests{use super::*;
 #[test]fn bombers_prioritize_carrier_even_when_escort_is_closer(){let mut f=flight(true);f.crashed=true;let mut c=Combat::default();start(&mut c,&f,1,1);let mut n=c.naval.take().unwrap();n.raiders.truncate(2);for r in &mut n.raiders{r.role=Role::Bomber;r.bay=1.;r.next_shot=0.;r.stores=2;let t=&mut c.targets[r.slot];t.position=carrier::world(DVec3::new(-20000.,3500.,0.),0.);t.velocity=carrier::rotation(0.)*DVec3::X*230.;t.orientation=facing(t.velocity.normalize(),t.position.normalize());t.dynamics.mach=t.velocity.length()/atmosphere(t.position.length()-RADIUS).3;}n.hostile_launches(&c,&f);assert_eq!(n.rounds.len(),2);assert!(n.rounds.iter().all(|m|m.target==TargetId::Ship(0)));}
}
#[cfg(test)]mod bay_interlock_tests{use super::*;
 #[test]fn bomber_release_waits_for_open_bays_and_empty_bays_close(){
  let mut f=flight(true);f.crashed=true;let mut c=Combat::default();start(&mut c,&f,1,1);let mut n=c.naval.take().unwrap();n.raiders.retain(|r|r.role==Role::Bomber);
  for r in &mut n.raiders{r.next_shot=0.;r.bay=0.;let t=&mut c.targets[r.slot];t.position=carrier::world(DVec3::new(1000.,3000.,-18000.),0.);t.velocity=carrier::rotation(0.)*DVec3::Z*220.;t.orientation=facing(t.velocity.normalize(),t.position.normalize());ai::initialize(t);}
  n.hostile_launches(&c,&f);assert!(n.rounds.is_empty());
  for _ in 0..240{n.pilots(&mut c,&f,1./120.);n.clock+=1./120.;}
  assert!(n.raiders.iter().all(|r|r.bay>0.99));n.hostile_launches(&c,&f);assert_eq!(n.rounds.len(),2);
  for m in &n.rounds{let TargetId::Aircraft(id)=m.source else{panic!("aircraft source")};let r=n.raiders.iter().find(|r|r.id==id).unwrap();let t=&c.targets[r.slot];assert!(m.position.distance(t.position+t.orientation*super::super::bomber::release_station(2))<1e-6);}
  for r in &mut n.raiders{r.stores=0;r.withdrew=true;}for _ in 0..240{n.pilots(&mut c,&f,1./120.);}assert!(n.raiders.iter().all(|r|r.bay<0.01));
 }
}

#[cfg(test)] mod countermeasure_tests {
 use super::*;
 #[test] fn raider_dispenses_finite_pairs_with_cooldown_and_ship_chaff_requires_detection() {
  let mut f=flight(true);let mut c=Combat::default();start(&mut c,&f,100,1);let mut n=c.naval.take().unwrap();
  let r=&n.raiders[0];let id=r.id;let slot=r.slot;let p=c.targets[slot].position;
  assert!(n.round(TargetId::Ship(1),TargetId::Aircraft(id),Kind::Standard,p-DVec3::Z*2000.,DVec3::Z*500.));
  n.countermeasures(&mut c,&f);assert_eq!(n.raiders[0].chaff,11);
  assert_eq!(c.fight.as_ref().unwrap().decoys.len(),2);
  n.clock=0.3;n.countermeasures(&mut c,&f);assert_eq!(c.fight.as_ref().unwrap().decoys.len(),2);
  n.clock=6.1;n.countermeasures(&mut c,&f);assert_eq!(n.raiders[0].chaff,10);
  n.rounds.clear();let p=carrier::ship_world(&f,0,DVec3::new(0.,30.,-3000.),f.elapsed);
  assert!(n.round(TargetId::Aircraft(id),TargetId::Ship(0),Kind::AntiShip,p,DVec3::Z*250.));
  n.clock=7.;n.countermeasures(&mut c,&f);assert_eq!(n.ship_chaff[0],16);
  let m=&n.rounds[0];n.missile_tracks.push(Track{id:m.id,position:p,velocity:m.velocity,seen:n.clock,hits:2,role:None});
  n.clock=7.3;n.countermeasures(&mut c,&f);assert_eq!(n.ship_chaff[0],15);
  assert!(c.fight.as_ref().unwrap().decoys.iter().any(|d|d.owner.is_none()&&d.kind==Weapon::Radar));
  n.clock=7.6;n.countermeasures(&mut c,&f);assert_eq!(n.ship_chaff[0],15);
  // Service and player respawn must not silently refill fleet expendables.
  f.landed=false;assert!(!n.service(&mut c,&mut f));assert_eq!(n.ship_chaff[0],15);
 }
}

#[cfg(test)] mod larger_bomber_load_tests {
 use super::*;
 #[test] fn each_bomber_releases_full_load_before_withdrawing_and_opening_the_next_raid() {
  let mut f=flight(true);f.crashed=true;let mut c=Combat::default();start(&mut c,&f,1,1);let mut n=c.naval.take().unwrap();
  for r in &mut n.raiders {r.next_shot=0.;r.aam=0;if r.role!=Role::Bomber {r.stores=0;continue;}
   assert_eq!(r.stores,BOMBER_MISSILES);assert_eq!(c.targets[r.slot].dynamics.bomber_stores,BOMBER_MISSILES);
   r.bay=1.;let t=&mut c.targets[r.slot];t.position=carrier::world(DVec3::new(-20000.,3500.,0.),0.);
   t.velocity=carrier::rotation(0.)*DVec3::X*230.;t.orientation=facing(t.velocity.normalize(),t.position.normalize());
   t.dynamics.mach=t.velocity.length()/atmosphere(t.position.length()-RADIUS).3;
  }
  for shot in 0..BOMBER_MISSILES {n.clock=shot as f64*5.;n.hostile_launches(&c,&f);
   assert_eq!(n.rounds.len(),(shot as usize+1)*2);
   assert!(n.raiders.iter().filter(|r|r.role==Role::Bomber).all(|r|r.withdrew==(shot+1==BOMBER_MISSILES)));
   if shot+1<BOMBER_MISSILES {n.spawn(&mut c,&f);assert_eq!(n.waves,1);}
  }
  n.spawn(&mut c,&f);assert_eq!(n.waves,2);
 }
}
