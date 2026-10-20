//! Shared location-based aircraft damage. Normalized structural/fragment doses
//! are game assumptions; aerodynamic increments preserve force/lever-arm signs.
use super::Weapon;
use crate::physics::{AircraftDamage, WING_AREA};
use bevy::math::DVec3;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum SurfaceState {
    #[default]
    Operational,
    Degraded,
    Jammed(f64),
    Detached,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Severity {
    Minimal,
    Critical,
    Destruction,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Component {
    Skin,
    Fuel,
    Engine,
    Hydraulics,
    Radar,
    Avionics,
    Wing(usize),
    Surface(usize),
    Nozzle(usize),
    LiftFan,
}
#[derive(Clone, Debug)]
pub struct DamageEvent {
    pub severity: Severity,
    pub component: Component,
    pub new_failures: Vec<Component>,
    pub breakup: bool,
    pub fire: bool,
}
impl DamageEvent {
    pub fn label(&self) -> String {
        format!(
            "{:?} / {:?}{} / {} FAILURE(S){}",
            self.severity,
            self.component,
            if self.breakup { " / AIRFRAME LOST" } else { "" },
            self.new_failures.len(),
            if self.fire { " / FIRE" } else { "" }
        )
    }
}
pub const SURFACE_POINTS: [DVec3; 9] = [
    DVec3::new(-1.35, 0.32, -2.65),
    DVec3::new(-1.58, -0.16, 2.75),
    DVec3::new(-3.1, -0.19, 2.97),
    DVec3::new(-0.6, 0.4, 4.0),
    DVec3::new(1.35, 0.32, -2.65),
    DVec3::new(1.58, -0.16, 2.75),
    DVec3::new(3.1, -0.19, 2.97),
    DVec3::new(0.6, 0.4, 4.0),
    DVec3::new(0.0, 1.5, 4.0),
];
pub fn random(state: &mut u64) -> f64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut x = *state;
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((x ^ (x >> 31)) >> 11) as f64 / (1u64 << 53) as f64
}
pub fn probabilities(distance: f64) -> [f64; 3] {
    let t = distance.clamp(0.0, 1.0);
    let (a, b, u) = if t <= 0.5 {
        ([0.05, 0.25, 0.70], [0.25, 0.50, 0.25], t * 2.0)
    } else {
        ([0.25, 0.50, 0.25], [0.55, 0.40, 0.05], (t - 0.5) * 2.0)
    };
    let u = u * u * (3.0 - 2.0 * u);
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * u)
}
pub fn missile(
    d: &mut AircraftDamage,
    hull: &mut f64,
    body: DVec3,
    kind: Weapon,
    angles: [f64; crate::aircraft::SURFACES],
) -> DamageEvent {
    resolve_missile(d, hull, body, kind, angles, false)
}
/// Enemy vulnerability tuning: direct hits are overwhelmingly lethal, while
/// grazing proximity bursts retain a meaningful chance of component failures.
pub fn enemy_probabilities(distance: f64) -> [f64; 3] {
    let base = probabilities(distance);
    let destruction = (0.80 + 0.17 * (1.0 - distance.clamp(0.0, 1.0))).clamp(0.80, 0.97);
    let surviving = 1.0 - destruction;
    let ratio = base[0] / (base[0] + base[1]);
    [surviving * ratio, surviving * (1.0 - ratio), destruction]
}
pub fn enemy_missile(
    d: &mut AircraftDamage,
    hull: &mut f64,
    body: DVec3,
    kind: Weapon,
    angles: [f64; crate::aircraft::SURFACES],
) -> DamageEvent {
    resolve_missile(d, hull, body, kind, angles, true)
}
fn resolve_missile(
    d: &mut AircraftDamage,
    hull: &mut f64,
    body: DVec3,
    kind: Weapon,
    angles: [f64; crate::aircraft::SURFACES],
    enemy: bool,
) -> DamageEvent {
    let distance = crate::airframe::distance_to_aircraft(body,d.aircraft);
    let odds = if enemy {
        enemy_probabilities(distance / kind.fuse())
    } else {
        probabilities(distance / kind.fuse())
    };
    let roll = random(&mut d.rng);
    let severity = if roll < odds[0] {
        Severity::Minimal
    } else if roll < odds[0] + odds[1] {
        Severity::Critical
    } else {
        Severity::Destruction
    };
    d.last_hit = 0.0;
    let mut component = Component::Skin;
    let mut failures = Vec::new();
    match severity {
        Severity::Minimal => {
            reduce(hull, 0.025);
            // Cosmetic damage cannot disable a new system; leaking fuel is persistent.
            if random(&mut d.rng) < 0.65 {
                d.fuel_leak += 0.8;
                component = Component::Fuel;
            }
        }
        Severity::Critical => {
            let mut candidates = vec![
                (Component::Engine, DVec3::new(0.0, 0.0, 4.8)),
                (Component::Hydraulics, DVec3::ZERO),
                (Component::Radar, DVec3::new(0.0, 0.0, -5.8)),
                (Component::Avionics, DVec3::new(0.0, 0.3, -3.5)),
                (Component::Wing(0), DVec3::new(-2.4, 0.0, 1.0)),
                (Component::Wing(1), DVec3::new(2.4, 0.0, 1.0)),
            ];
            if d.aircraft!=crate::aircraft::Aircraft::Gripen {
                let s=d.aircraft.spec();
                candidates[0].1=d.aircraft.engine_positions()[usize::from(body.x>0.)];
                candidates[2].1=DVec3::new(0.,0.,-s.length*0.43);
                candidates[3].1=DVec3::new(0.,0.3,-s.length*0.25);
                candidates[4].1=DVec3::new(-s.span*0.23,0.,s.length*0.05);
                candidates[5].1=DVec3::new(s.span*0.23,0.,s.length*0.05);
                if d.aircraft==crate::aircraft::Aircraft::F22 {for(i,p)in d.aircraft.engine_positions().iter().enumerate().take(s.engines){candidates.push((Component::Nozzle(i),*p+DVec3::Z*0.7));}}
                if d.aircraft==crate::aircraft::Aircraft::F35B {candidates.push((Component::LiftFan,DVec3::new(0.,0.4,-2.6)));}
            }
            candidates.extend(
                crate::aircraft::surface_points(d.aircraft)
                    .iter()
                    .take(crate::aircraft::count(d.aircraft))
                    .enumerate()
                    .filter(|(i,_)|d.aircraft==crate::aircraft::Aircraft::Gripen||d.aircraft.panels()[*i].area>0.)
                    .map(|(i, p)| (Component::Surface(i), *p)),
            );
            component = candidates
                .into_iter()
                .min_by(|a, b| {
                    body.distance_squared(a.1)
                        .total_cmp(&body.distance_squared(b.1))
                })
                .unwrap()
                .0;
            blast_scaled(d, hull, body, 0.12);
            reduce(hull, 0.12);
            match component {
                Component::Engine => {
                    if d.aircraft==crate::aircraft::Aircraft::Gripen {d.engine=0.;}else{
                        let n=d.aircraft.spec().engines;let i=if n==1{0}else{usize::from(body.x>0.)};d.engines[i]=0.;d.nozzles[i]=0.;d.engine=d.engines[..n].iter().sum::<f64>()/n as f64;
                    }
                    d.fire = d.fire.max(0.65);
                }
                Component::Hydraulics => d.hydraulics = 0.0,
                Component::Nozzle(i)=>d.nozzles[i]=0.,
                Component::LiftFan=>{d.lift_fan=0.;d.fire=d.fire.max(0.5);},
                Component::Radar => d.radar = 0.0,
                Component::Avionics => d.avionics = 0.0,
                Component::Wing(side) => {
                    d.wings[side] = (d.wings[side] - 0.6).max(0.0);
                    reduce(&mut d.structure[side + 1], 0.45);
                    d.fuel_leak += 1.5;
                    d.fire = d.fire.max(0.5);
                }
                Component::Surface(i) => {
                    if d.surface_state[i] != SurfaceState::Detached {
                        d.surface_state[i] = if random(&mut d.rng) < 0.5 {
                            SurfaceState::Jammed(angles[i])
                        } else {
                            SurfaceState::Detached
                        };
                        if d.surface_state[i] == SurfaceState::Detached {
                            d.surfaces[i] = 0.0;
                        }
                    }
                }
                _ => {}
            }
            failures.push(component);
        }
        Severity::Destruction => {
            *hull = 0.0;
            d.structure[3] = 0.0;
            d.fire = 1.0;
        }
    }
    DamageEvent {
        severity,
        component,
        new_failures: failures,
        breakup: fatal(d, *hull),
        fire: d.fire > 0.0,
    }
}

pub fn closest_relative(start: DVec3, end: DVec3) -> (DVec3, f64) {
    let delta = end - start;
    let t = (-start.dot(delta) / delta.length_squared().max(1e-12)).clamp(0.0, 1.0);
    (start + delta * t, t)
}
fn reduce(value: &mut f64, amount: f64) {
    *value = (*value - amount.max(0.0)).clamp(0.0, 1.0);
}
pub fn projectile(d: &mut AircraftDamage, hull: &mut f64, body: DVec3, dose: f64) {
    let dose = dose.clamp(0.0, 1.5);
    if d.aircraft!=crate::aircraft::Aircraft::Gripen {
        d.last_hit=0.;reduce(hull,dose*0.32);
        for(i,p)in d.aircraft.panels().iter().enumerate().filter(|(_,p)|p.area>0.) {
            let falloff=(1.-body.distance(p.center)/1.5).clamp(0.,1.);reduce(&mut d.surfaces[i],dose*falloff*2.5);
        }
        for(i,p)in d.aircraft.engine_positions().iter().enumerate().take(d.aircraft.spec().engines){let k=(1.-body.distance(*p)/2.).clamp(0.,1.);reduce(&mut d.engines[i],dose*k*1.6);reduce(&mut d.nozzles[i],dose*k*2.);}
        let n=d.aircraft.spec().engines;d.engine=d.engines[..n].iter().sum::<f64>()/n as f64;
        if body.z < -d.aircraft.spec().length*0.25 {reduce(&mut d.radar,dose*1.2);reduce(&mut d.avionics,dose*0.8);}else{reduce(&mut d.hydraulics,dose*0.3);d.fuel_leak+=dose*0.8;}
        if d.aircraft==crate::aircraft::Aircraft::F35B && body.distance(DVec3::new(0.,0.,-2.6))<1.5{reduce(&mut d.lift_fan,dose*2.);}
        if d.engine<0.7{d.fire=d.fire.max(0.4);}return;
    }
    d.last_hit = 0.0;
    reduce(hull, dose * 0.32);
    if body.x.abs() > 0.95 {
        let side = usize::from(body.x > 0.0);
        let index = side * 4
            + if body.z < 0.0 {
                0
            } else if body.x.abs() > 2.4 {
                2
            } else {
                1
            };
        reduce(&mut d.surfaces[index], dose * 2.5);
        reduce(&mut d.hydraulics, dose * 0.25);
        if body.z > 0.0 {
            reduce(&mut d.wings[side], dose * 1.4);
            reduce(&mut d.structure[side + 1], dose * 1.6);
            d.fuel_leak += dose * 0.5;
        }
    } else if body.z > 2.0 {
        reduce(&mut d.engine, dose * 1.6);
        reduce(&mut d.structure[4], dose * 0.5);
        reduce(&mut d.surfaces[8], dose * 1.2);
        if d.engine < 0.55 {
            d.fire = (d.fire + dose * 0.6).min(1.0);
        }
    } else if body.z < -1.0 {
        reduce(&mut d.avionics, dose * 1.6);
        reduce(&mut d.radar, dose * 1.8);
        reduce(&mut d.structure[0], dose * 0.6);
    } else {
        reduce(&mut d.hydraulics, dose * 1.4);
        reduce(&mut d.structure[3], dose * 0.8);
        d.fuel_leak += dose * 1.2;
        if d.fuel_leak > 0.35 {
            d.fire = (d.fire + dose * 0.5).min(1.0);
        }
    }
}
pub fn enemy_projectile(d: &mut AircraftDamage, hull: &mut f64, body: DVec3, dose: f64) {
    projectile(d, hull, body, dose * 4.0);
}
pub fn blast(d: &mut AircraftDamage, hull: &mut f64, body: DVec3, kind: Weapon) {
    blast_scaled(d, hull, body, if kind == Weapon::Heat { 1.0 } else { 1.3 });
}
pub fn blast_scaled(d: &mut AircraftDamage, hull: &mut f64, body: DVec3, scale: f64) {
    d.last_hit = 0.0;
    let dose = |point: DVec3| (scale * 16.0 / (body.distance(point) + 2.0).powi(2)).clamp(0.0, 1.4);
    if d.aircraft!=crate::aircraft::Aircraft::Gripen {
        let core=dose(DVec3::ZERO);reduce(hull,core*0.5);reduce(&mut d.structure[3],core*0.5);reduce(&mut d.hydraulics,core*0.5);
        for(i,p)in d.aircraft.panels().iter().enumerate().filter(|(_,p)|p.area>0.){reduce(&mut d.surfaces[i],dose(p.center)*0.8);}
        for(i,p)in d.aircraft.engine_positions().iter().enumerate().take(d.aircraft.spec().engines){reduce(&mut d.engines[i],dose(*p));reduce(&mut d.nozzles[i],dose(*p));}
        let n=d.aircraft.spec().engines;d.engine=d.engines[..n].iter().sum::<f64>()/n as f64;
        if d.aircraft==crate::aircraft::Aircraft::F35B{reduce(&mut d.lift_fan,dose(DVec3::new(0.,0.,-2.6)));}
        d.fuel_leak+=core;return;
    }
    let core = dose(DVec3::ZERO);
    reduce(hull, core * 0.62);
    reduce(&mut d.structure[3], core * 0.85);
    reduce(&mut d.hydraulics, core * 0.8);
    d.fuel_leak += core * 0.9;
    let nose = dose(DVec3::new(0.0, 0.2, -3.8));
    reduce(&mut d.avionics, nose * 0.9);
    reduce(&mut d.radar, nose * 1.1);
    reduce(&mut d.structure[0], nose * 0.55);
    let tail = dose(DVec3::new(0.0, 0.5, 4.5));
    reduce(&mut d.engine, tail * 1.0);
    reduce(&mut d.structure[4], tail * 0.7);
    reduce(&mut d.surfaces[8], tail);
    for side in 0..2 {
        let x = if side == 0 { -2.5 } else { 2.5 };
        let wing = dose(DVec3::new(x, 0.0, 1.8));
        reduce(&mut d.wings[side], wing * 0.72);
        reduce(&mut d.structure[side + 1], wing * 0.85);
        for panel in side * 4..side * 4 + 4 {
            reduce(&mut d.surfaces[panel], wing * 0.8);
        }
    }
    if tail > 0.35 || core > 0.35 {
        d.fire = (d.fire + tail * 0.45 + core * 0.25).min(1.0);
    }
}
pub fn advance(d: &mut AircraftDamage, hull: &mut f64, dt: f64, engine_on: bool, fuel: bool) {
    d.last_hit += dt;
    for i in 0..crate::aircraft::count(d.aircraft) {
        if matches!(
            d.surface_state[i],
            SurfaceState::Operational | SurfaceState::Degraded
        ) {
            d.surface_state[i] = if d.surfaces[i] <= 0.02 {
                SurfaceState::Detached
            } else if d.surfaces[i] < 0.99 {
                SurfaceState::Degraded
            } else {
                SurfaceState::Operational
            };
        }
    }
    if d.fire > 0.0 {
        // Cutting fuel/engine reduces an engine-fed fire, but damage remains.
        if !fuel || (!engine_on && d.fuel_leak <= 0.0) {
            d.fire = (d.fire - dt * 0.12).max(0.0);
        } else {
            d.fire = (d.fire + dt * 0.008).min(1.0);
        }
        if fuel && d.fuel_leak > 0.0 && d.fire > 0.3 {
            if !d.explosion_at.is_finite() {
                d.explosion_at = 5.0 - (1.0 - random(&mut d.rng)).ln() / 0.001;
            }
            d.fire_age += dt;
            if d.fire_age >= d.explosion_at {
                *hull = 0.0;
                d.structure[3] = 0.0;
            }
        } else {
            d.fire_age = 0.0;
            d.explosion_at = f64::INFINITY;
        }
    } else {
        d.fire_age = 0.0;
        d.explosion_at = f64::INFINITY;
    }
}
pub fn fatal(d: &AircraftDamage, hull: f64) -> bool {
    hull < 0.10
        || d.structure[3] < 0.10
        || d.structure[1] < 0.08
        || d.structure[2] < 0.08
        || d.structure[4] < 0.08
}
pub fn aero(d: &AircraftDamage, flow: DVec3, q: f64, cl: f64) -> (DVec3, DVec3) {
    if d.aircraft!=crate::aircraft::Aircraft::Gripen {
        let lift=DVec3::X.cross(flow.normalize_or_zero()).normalize_or_zero();let mut force=DVec3::ZERO;let mut moment=DVec3::ZERO;let s=d.aircraft.spec();
        for side in 0..2 {let lost=-lift*(q*s.area*cl*0.4*(1.-d.wings[side]));let arm=DVec3::new(if side==0{-s.span*0.24}else{s.span*0.24},0.,s.length*0.04);force+=lost;moment+=arm.cross(lost);}
        for(i,p)in d.aircraft.panels().iter().enumerate().filter(|(_,p)|p.area>0.){if d.surface_state[i]==SurfaceState::Detached && p.normal==DVec3::Y {let lost=-lift*(q*cl*p.area);force+=lost;moment+=p.center.cross(lost);}}
        force-=flow.normalize_or_zero()*(q*s.area*0.04*(2.-d.wings[0]-d.wings[1]));return(force,moment);
    }
    let lift = DVec3::X.cross(flow.normalize_or_zero()).normalize_or_zero();
    let mut force = DVec3::ZERO;
    let mut moment = DVec3::ZERO;
    for side in 0..2 {
        let lost = -lift * (q * WING_AREA * cl * 0.4 * (1.0 - d.wings[side]));
        let arm = DVec3::new(if side == 0 { -2.3 } else { 2.3 }, 0.0, 0.7);
        force += lost;
        moment += arm.cross(lost);
    }
    // Neutral panel lift belongs to the whole-airframe polar. Remove that
    // contribution as well as its deflection increment when skin is detached.
    for (i, area) in [1.5, 1.8, 1.5, 0.0, 1.5, 1.8, 1.5, 0.0, 0.0]
        .into_iter()
        .enumerate()
    {
        if d.surface_state[i] == SurfaceState::Detached {
            let lost = -lift * (q * cl * area * d.wings[usize::from(i >= 4)]);
            force += lost;
            moment += SURFACE_POINTS[i].cross(lost);
        }
    }
    // Torn skin adds parasitic drag; it cannot create propulsive force.
    force -= flow.normalize_or_zero() * (q * WING_AREA * 0.04 * (2.0 - d.wings[0] - d.wings[1]));
    (force, moment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::default;
    #[test]
    fn enemy_missile_odds_are_mostly_lethal_at_every_burst_distance() {
        for step in 0..=100 {
            let p = enemy_probabilities(step as f64 / 100.);
            assert!((p.iter().sum::<f64>() - 1.).abs() < 1e-12);
            assert!(p[2] >= 0.8 && p[2] <= 0.97);
        }
        for kind in [Weapon::Heat, Weapon::Radar] {
            let mut destroyed = 0;
            let mut survivors = 0;
            for seed in 0..5000 {
                let mut d = AircraftDamage {
                    rng: seed,
                    ..default()
                };
                let mut h = 1.;
                let e = enemy_missile(&mut d, &mut h, DVec3::ZERO, kind, [0.; crate::aircraft::SURFACES]);
                destroyed += usize::from(e.breakup);
                survivors += usize::from(!e.breakup);
                let mut replay = AircraftDamage {
                    rng: seed,
                    ..default()
                };
                let mut hull = 1.;
                let again = enemy_missile(&mut replay, &mut hull, DVec3::ZERO, kind, [0.; crate::aircraft::SURFACES]);
                assert_eq!(e.severity, again.severity);
                assert_eq!(h, hull);
            }
            assert!((destroyed as f64 / 5000. - 0.97).abs() < 0.015);
            assert!(survivors > 0);
        }
        assert_eq!(probabilities(0.5), [0.25, 0.50, 0.25]);
    }
    #[test]
    fn stronger_enemy_cannon_remains_local_and_cumulative() {
        let mut normal = AircraftDamage::default();
        let mut enemy = normal.clone();
        let (mut a, mut b) = (1., 1.);
        let tail = DVec3::new(0., 0., 4.8);
        projectile(&mut normal, &mut a, tail, 0.075);
        enemy_projectile(&mut enemy, &mut b, tail, 0.075);
        assert!(((1. - b) / (1. - a) - 4.).abs() < 1e-10);
        assert!(enemy.engine < normal.engine);
        assert_eq!(enemy.radar, 1.);
        for _ in 0..20 {
            enemy_projectile(&mut enemy, &mut b, tail, 0.075);
        }
        assert_eq!(enemy.engine, 0.);
        assert!(fatal(&enemy, b));
    }
    #[test]
    fn probability_anchors_and_seeded_distribution_match_specification() {
        for (distance, expected) in [
            (0.0, [0.05, 0.25, 0.70]),
            (0.5, [0.25, 0.50, 0.25]),
            (1.0, [0.55, 0.40, 0.05]),
        ] {
            let actual = probabilities(distance);
            for i in 0..3 {
                assert!((actual[i] - expected[i]).abs() < 1e-12);
            }
            let mut state = 491u64;
            let mut count = [0usize; 3];
            for _ in 0..20000 {
                let r = random(&mut state);
                count[if r < actual[0] {
                    0
                } else if r < actual[0] + actual[1] {
                    1
                } else {
                    2
                }] += 1;
            }
            for i in 0..3 {
                assert!((count[i] as f64 / 20000.0 - expected[i]).abs() < 0.015);
            }
        }
    }
    #[test]
    fn missiles_disable_local_systems_and_can_jam_or_detach_controls() {
        let mut jam = false;
        let mut detached = false;
        let mut engine = false;
        let mut minimal = false;
        let mut fatal_hit = false;
        for seed in 0..100 {
            for point in [SURFACE_POINTS[2], DVec3::new(0.0, 0.0, 5.0)] {
                let mut d = AircraftDamage {
                    rng: seed,
                    ..default()
                };
                let mut h = 1.0;
                let event = missile(&mut d, &mut h, point, Weapon::Heat, [0.19; crate::aircraft::SURFACES]);
                match event.severity {
                    Severity::Minimal => {
                        minimal = true;
                        assert_eq!(d.engine, 1.0);
                        assert!(
                            d.surface_state
                                .iter()
                                .all(|s| *s == SurfaceState::Operational)
                        );
                    }
                    Severity::Critical => {
                        assert!(!event.breakup);
                        assert_eq!(event.new_failures.len(), 1);
                        if event.component == Component::Engine {
                            engine = true;
                            assert_eq!(d.engine, 0.0);
                        }
                        if let SurfaceState::Jammed(a) = d.surface_state[2] {
                            jam = true;
                            assert_eq!(a, 0.19);
                        }
                        if d.surface_state[2] == SurfaceState::Detached {
                            detached = true;
                            assert_eq!(d.surfaces[2], 0.0);
                        }
                    }
                    Severity::Destruction => {
                        fatal_hit = true;
                        assert!(event.breakup);
                    }
                }
            }
        }
        assert!(jam && detached && engine && minimal && fatal_hit);
    }
    #[test]
    fn fire_hazard_is_timestep_independent_and_visual_fire_preserves_health() {
        let mut d = AircraftDamage {
            fire: 0.7,
            fuel_leak: 0.8,
            rng: 20,
            ..default()
        };
        let mut h = 0.6;
        advance(&mut d, &mut h, 0.01, true, true);
        assert!(d.explosion_at >= 5.0);
        assert_eq!(h, 0.6);
        let mut fine = d.clone();
        let mut coarse = d.clone();
        let mut hf = h;
        let mut hc = h;
        fine.explosion_at = 6.0;
        coarse.explosion_at = 6.0;
        for _ in 0..600 {
            advance(&mut fine, &mut hf, 0.01, true, true);
        }
        for _ in 0..60 {
            advance(&mut coarse, &mut hc, 0.1, true, true);
        }
        assert!(fatal(&fine, hf) && fatal(&coarse, hc));
        d.fire = 0.0;
        advance(&mut d, &mut h, 1.0, false, false);
        assert!(d.explosion_at.is_infinite());
        assert_eq!(d.fire_age, 0.0);
    }
    #[test]
    fn nearest_point_retains_impact_side_when_endpoint_passes_target() {
        let (point, t) =
            closest_relative(DVec3::new(-100.0, 5.0, 0.0), DVec3::new(100.0, 5.0, 0.0));
        assert_eq!(point, DVec3::Y * 5.0);
        assert_eq!(t, 0.5);
    }
    #[test]
    fn proximity_damage_falls_off_and_can_be_survived() {
        let mut near = AircraftDamage::default();
        let mut far = near.clone();
        let mut hn = 1.0;
        let mut hf = 1.0;
        blast(&mut near, &mut hn, DVec3::new(-6.0, 1.0, 2.0), Weapon::Heat);
        blast(&mut far, &mut hf, DVec3::new(-13.0, 1.0, 2.0), Weapon::Heat);
        assert!(hn < hf && near.wings[0] < far.wings[0]);
        assert!(near.wings[0] < near.wings[1]);
        assert!(!fatal(&near, hn));
        assert!(near.engine < 1.0 && near.hydraulics < 1.0);
    }
    #[test]
    fn damage_is_cumulative_and_critical_structural_hits_are_lethal() {
        let mut d = AircraftDamage::default();
        let mut h = 1.0;
        projectile(&mut d, &mut h, DVec3::new(0.0, 0.0, 4.0), 0.1);
        assert!(d.engine < 1.0 && !fatal(&d, h));
        for _ in 0..20 {
            projectile(&mut d, &mut h, DVec3::ZERO, 0.1);
        }
        assert!(fatal(&d, h));
    }
    #[test]
    fn engine_cut_extinguishes_fire_without_repairing_damage() {
        let mut d = AircraftDamage {
            fire: 0.7,
            engine: 0.4,
            ..Default::default()
        };
        let mut h = 0.8;
        for _ in 0..1000 {
            advance(&mut d, &mut h, 0.01, false, true);
        }
        assert_eq!(d.fire, 0.0);
        assert_eq!(d.engine, 0.4);
    }
    #[test]
    fn lost_wing_area_reduces_lift_and_creates_signed_roll_moment() {
        let mut d = AircraftDamage::default();
        d.wings[0] = 0.4;
        let (force, moment) = aero(&d, -DVec3::Z * 200.0, 20000.0, 0.5);
        assert!(force.y < 0.0 && force.z > 0.0 && moment.z > 0.0);
        d.wings = [1.0, 0.4];
        assert!(aero(&d, -DVec3::Z * 200.0, 20000.0, 0.5).1.z < 0.0);
    }
}
