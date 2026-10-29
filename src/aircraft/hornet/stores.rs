//! Optional 480-US-gallon tanks. Capacity from NAVAIR; dry mass, drag and
//! release impulse are explicitly estimated. Fuel is independent of internal fuel.
use super::*;
pub const CAPACITY: f64 = 480. * 3.785411784 * 0.8; // kg; estimated JP-5 density .8 kg/L
pub const DRY: f64 = 130.;
pub const TANK_RADIUS: f64 = 0.432;
pub const DRAG: f64 = 0.006;
pub const STATIONS: [DVec3; 3] = [
    DVec3::new(-2.28, -0.98, 1.10),
    DVec3::new(2.28, -0.98, 1.10),
    DVec3::new(0., -1.18, 0.70),
];
#[derive(Clone, Debug)]
pub struct Dropped {
    pub position: DVec3,
    pub velocity: DVec3,
    pub orientation: DQuat,
    pub age: f64,
    pub mass: f64,
    pub alive: bool,
}
#[derive(Clone, Debug, Default)]
pub struct Tanks {
    pub loadout: bool,
    pub fuel: [Option<f64>; 3],
    pub dropped: [Option<Dropped>; 3],
}
pub fn fuel(f: &Flight) -> f64 {
    f.hornet.tanks.fuel.iter().flatten().sum()
}
pub fn mass(f: &Flight) -> f64 {
    f.hornet.tanks.fuel.iter().flatten().map(|v| DRY + v).sum()
}
pub fn fit(f: &mut Flight) {
    f.hornet.tanks = Tanks {
        loadout: true,
        fuel: [Some(CAPACITY); 3],
        ..Default::default()
    };
}
/// Initial loadout only: preserve lift coefficient by increasing spawn speed,
/// then resolve trim at the loaded mass. Runtime jettison never imposes motion.
pub fn configure_scenario(f: &mut Flight) {
    let old_mass = f.mass();
    fit(f);
    if !f.landed {
        let ratio = (f.mass() / old_mass).sqrt();
        f.velocity = f.wind + (f.velocity - f.wind) * ratio;
        f.airspeed = (f.velocity - f.wind).length();
        f.mach = f.airspeed / atmosphere(f.altitude()).3;
        let gamma = (f.vertical_speed() / f.airspeed.max(1.))
            .clamp(-0.3, 0.3)
            .asin();
        if let Some(trim) = super::trim::solve(f, f.altitude(), f.airspeed, gamma, 0.) {
            let pitch = f.pitch();
            f.orientation *= DQuat::from_rotation_x(trim.alpha + gamma - pitch);
            super::trim::apply(f, trim);
        }
    }
}
pub fn burn(f: &mut Flight, kg: f64) {
    let available = fuel(f);
    let external = kg.max(0.).min(available);
    if available > 0. {
        for tank in f.hornet.tanks.fuel.iter_mut().flatten() {
            *tank = (*tank * (1. - external / available)).max(0.);
        }
    }
    f.fuel = (f.fuel - (kg.max(0.) - external)).max(0.);
}
pub fn jettison(f: &mut Flight, only: Option<usize>) -> usize {
    let mut count = 0;
    let mut impulse = DVec3::ZERO;
    let mut angular_impulse = DVec3::ZERO;
    for (i, station) in STATIONS.iter().enumerate() {
        if only.is_some_and(|j| j != i) {
            continue;
        }
        if let Some(fuel) = f.hornet.tanks.fuel[i].take() {
            let recoil = DVec3::Y * (2. * (DRY + fuel));
            impulse += recoil;
            angular_impulse += station.cross(recoil);
            f.hornet.tanks.dropped[i] = Some(Dropped {
                position: f.position + f.orientation * *station,
                velocity: f.velocity + f.orientation * (f.rates.cross(*station) - DVec3::Y * 2.),
                orientation: f.orientation,
                age: 0.,
                mass: DRY + fuel,
                alive: true,
            });
            count += 1;
        }
    }
    if count > 0 {
        let (cg, inertia) = super::mass::properties(f);
        f.velocity += f.orientation * impulse / f.mass();
        f.rates += inertia.inverse() * (angular_impulse - cg.cross(impulse));
    }
    count
}
pub fn drag(f: &Flight, cg: DVec3) -> Loads {
    let air = f.orientation.conjugate() * (f.velocity - f.wind);
    let mut loads = Loads::default();
    for (i, at) in STATIONS.iter().enumerate() {
        if f.hornet.tanks.fuel[i].is_some() {
            let arm = *at - cg;
            let v = air + f.rates.cross(arm);
            loads.add(
                -v.normalize_or_zero()
                    * (0.5 * f.density * v.length_squared() * config::AREA * DRAG),
                arm,
            );
        }
    }
    loads
}
pub fn advance(f: &mut Flight, dt: f64) {
    let mut dropped = std::mem::take(&mut f.hornet.tanks.dropped);
    for d in dropped.iter_mut().flatten().filter(|d| d.alive) {
        let air = d.velocity - f.wind;
        let rho = atmosphere(d.position.length() - RADIUS).2;
        d.velocity +=
            (-d.position.normalize() * 9.81 - air * air.length() * rho * 0.16 / d.mass) * dt;
        d.position += d.velocity * dt;
        d.orientation = (d.orientation
            * DQuat::from_scaled_axis(DVec3::new(0.13, 0.04, 0.08) * dt))
        .normalize();
        d.age += dt;
        d.alive = d.age < 45.
            && d.position.length() - RADIUS - TANK_RADIUS > crate::carrier::ground(f, d.position);
    }
    f.hornet.tanks.dropped = dropped;
}

#[cfg(test)]
mod tests {
    use super::*;
    fn flight() -> Flight {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Hornet);
        f
    }
    #[test]
    fn tank_fuel_mass_inertia_drag_and_release_remain_consistent() {
        let mut f = flight();
        let base = f.mass();
        let base_drag = crate::aircraft::store_drag(&f);
        fit(&mut f);
        assert!((f.mass() - base - 3. * (CAPACITY + DRY)).abs() < 1e-6);
        assert!(crate::aircraft::store_drag(&f) > base_drag);
        let (cg, inertia) = super::super::mass::properties(&f);
        assert!(inertia.determinant() > 0.);
        let internal = f.fuel;
        burn(&mut f, 300.);
        assert_eq!(f.fuel, internal);
        assert!((fuel(&f) - (3. * CAPACITY - 300.)).abs() < 1e-6);
        let before = f.mass();
        assert_eq!(jettison(&mut f, Some(0)), 1);
        assert!((before - f.mass() - (DRY + CAPACITY - 100.)).abs() < 1e-6);
        assert!(super::super::mass::properties(&f).0.x > cg.x);
        assert!(drag(&f, super::super::mass::properties(&f).0).moment.y < 0.);
        assert_eq!(jettison(&mut f, Some(0)), 0);
        let d = f.hornet.tanks.dropped[0].as_ref().unwrap();
        assert!((d.velocity - f.velocity).dot(f.up()) < 0.);
        let old = d.position;
        advance(&mut f, 0.1);
        assert!(
            f.hornet.tanks.dropped[0]
                .as_ref()
                .unwrap()
                .position
                .distance(old)
                > 1.
        );
        assert_eq!(jettison(&mut f, None), 2);
        assert_eq!(fuel(&f), 0.);
        assert!((crate::aircraft::store_drag(&f) - base_drag).abs() < 1e-9);
    }
    #[test]
    fn engines_run_on_external_fuel_and_release_kick_conserves_linear_momentum() {
        let mut f = flight();
        fit(&mut f);
        f.fuel = 0.;
        f.engine = true;
        f.hornet.initialized = true;
        for e in &mut f.hornet.engines {
            e.spool = 0.8;
        }
        f.throttle = 0.8;
        let fuel_before = fuel(&f);
        let loads = super::super::engine::advance(&mut f, 0.1);
        assert!(loads.force.z < 0.);
        assert!(fuel(&f) < fuel_before);
        assert_eq!(f.fuel, 0.);
        f.rates = DVec3::ZERO;
        let momentum = f.velocity * f.mass();
        jettison(&mut f, None);
        let final_p = f.velocity * f.mass()
            + f.hornet
                .tanks
                .dropped
                .iter()
                .flatten()
                .map(|d| d.velocity * d.mass)
                .sum::<DVec3>();
        assert!(final_p.distance(momentum) < 1e-6);
    }
    #[test]
    fn wingtip_missiles_follow_the_fold_and_folded_launch_is_inhibited() {
        let mut f = flight();
        let old = crate::aircraft::station_for(&f, 0);
        f.systems.hornet_wing_fold = 1.;
        let left = crate::aircraft::station_for(&f, 0);
        let right = crate::aircraft::station_for(&f, 1);
        assert!(left.y > old.y + 1.5 && left.x > old.x + 1.);
        assert!((left.x + right.x).abs() < 1e-8 && (left.y - right.y).abs() < 1e-8);
        let mut combat = crate::combat::Combat::default();
        combat.start(&f, 1);
        assert_eq!(combat.cue(&f), Err("UNFOLD WINGS"));
    }
    #[test]
    fn loaded_trim_and_tank_release_remain_flyable() {
        let mut f = flight();
        configure_scenario(&mut f);
        f.paused = false;
        let mut e = Environment::default();
        e.wind_enabled = false;
        for _ in 0..600 {
            crate::physics::simulate(&mut f, &e, 1. / 120.);
        }
        assert!(
            !f.crashed && f.position.is_finite() && f.velocity.is_finite() && f.altitude() > 100.
        );
        let before = f.mass();
        jettison(&mut f, None);
        assert!(before - f.mass() > 4000.);
        for _ in 0..600 {
            crate::physics::simulate(&mut f, &e, 1. / 120.);
        }
        assert!(
            !f.crashed && f.position.is_finite() && f.rates.length() < 2. && f.altitude() > 100.
        );
    }
    #[test]
    fn released_tanks_stop_at_the_carrier_deck() {
        let mut f = crate::carrier::scenario(true);
        fit(&mut f);
        jettison(&mut f, None);
        for _ in 0..240 {
            advance(&mut f, 1. / 240.);
        }
        assert!(f.hornet.tanks.dropped.iter().flatten().all(|d| !d.alive));
    }
    #[test]
    fn tanks_feed_engines_and_exhaust_before_internal_fuel() {
        let mut f = flight();
        fit(&mut f);
        f.fuel = 10.;
        burn(&mut f, 3. * CAPACITY + 4.);
        assert!((f.fuel - 6.).abs() < 1e-6);
        assert!(fuel(&f) < 1e-6);
        burn(&mut f, 100.);
        assert_eq!(f.fuel, 0.);
    }
}
