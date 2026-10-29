//! Player aircraft definitions. Public dimensions guide the meshes; polars,
//! inertias and control laws are deliberately tunable gameplay estimates.
pub mod visual;
pub mod raptor;
pub mod hornet;
pub mod bays;
pub mod main_bay;
use crate::physics::*;
use bevy::{math::DVec3, prelude::*};

pub const SURFACES: usize = 16;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Aircraft {
    #[default]
    Gripen,
    Su27,
    F35B,
    F22,
    Hornet,
}
impl Aircraft {
    pub const ALL: [Self; 5] = [
        Self::Gripen,
        Self::Su27,
        Self::F35B,
        Self::F22,
        Self::Hornet,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Gripen => "JAS 39C GRIPEN",
            Self::Su27 => "SU-27SM FLANKER",
            Self::F35B => "F-35B LIGHTNING II",
            Self::F22 => "F-22A RAPTOR",
            Self::Hornet => "F/A-18F SUPER HORNET",
        }
    }
    pub fn slug(self) -> &'static str {
        match self {
            Self::Gripen => "gripen",
            Self::Su27 => "su27",
            Self::F35B => "f35b",
            Self::F22 => "f22",
            Self::Hornet => "fa18f",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        if s == "fa18e" { return Some(Self::Hornet); }
        Self::ALL.into_iter().find(|a| a.slug() == s)
    }
    pub fn spec(self) -> Spec {
        match self {
            Self::Gripen => Spec {
                length: 14.1,
                span: 8.4,
                area: 30.,
                empty: 6800.,
                fuel: 2400.,
                dry: 50400.,
                wet: 110400.,
                engines: 1,
                inertia: DVec3::new(95000., 115000., 18000.),
                g: 9.,
                alpha: 0.32,
                limit_mach: 2.05,
                gun: 120,
                gun_interval: 0.075,
                ammo: [2, 2],
            },
            Self::Su27 => Spec {
                length: 21.94,
                span: 14.7,
                area: 62.,
                empty: 16800.,
                fuel: 9400.,
                dry: 150000.,
                wet: 245000.,
                engines: 2,
                inertia: DVec3::new(220000., 290000., 58000.),
                g: 9.,
                alpha: 0.48,
                limit_mach: 2.35,
                gun: 150,
                gun_interval: 0.04,
                ammo: [4, 6],
            },
            Self::F35B => Spec {
                length: 15.67,
                span: 10.7,
                area: 42.7,
                empty: 14650.,
                fuel: 6100.,
                dry: 125000.,
                wet: 190000.,
                engines: 1,
                inertia: DVec3::new(125000., 160000., 42000.),
                g: 7.,
                alpha: 0.65,
                limit_mach: 1.65,
                gun: 220,
                gun_interval: 0.05,
                ammo: [2, 4],
            },
            Self::F22 => Spec {
                length: 18.92,
                span: 13.56,
                area: 78.,
                empty: 19700.,
                fuel: 8200.,
                dry: 232000.,
                wet: 312000.,
                engines: 2,
                inertia: DVec3::new(220000., 290000., 65000.),
                g: 9.,
                alpha: 1.05,
                limit_mach: 2.35,
                gun: 480,
                gun_interval: 0.02,
                ammo: [2, 6],
            },
            Self::Hornet => Spec {
                length: 18.31,
                span: 13.62,
                area: 46.5,
                empty: 14550.,
                fuel: 6500.,
                dry: 125000.,
                wet: 196000.,
                engines: 2,
                inertia: DVec3::new(170000., 210000., 44000.),
                g: 7.5,
                alpha: 0.62,
                limit_mach: 1.85,
                gun: 412,
                gun_interval: 0.02,
                ammo: [2, 6],
            },
        }
    }
    pub fn cabin_offset(self) -> Vec3 {
        if self == Self::Gripen {
            Vec3::ZERO
        } else {
            Vec3::new(
                0.,
                if self==Self::F22{-0.10}else{0.18},
                2.65 - self.spec().length as f32 * if self == Self::F35B { 0.32 } else { 0.255 },
            )
        }
    }
    pub fn engine_positions(self) -> [DVec3; 2] {
        let s = self.spec();
        let x = if s.engines == 2 {
            if self == Self::Su27 { 1.45 } else { 0.93 }
        } else {
            0.
        };
        [
            DVec3::new(-x, -0.12, s.length * 0.40),
            DVec3::new(x, -0.12, s.length * 0.40),
        ]
    }
    pub fn panels(self) -> [Panel; SURFACES] {
        if self==Self::F22{return raptor::panels();}
        let s = self.spec();
        let z = s.length * 0.34;
        let wing_z = s.length * 0.12;
        let wing_x = s.span * 0.30;
        let mut p = [Panel::empty(); SURFACES];
        for (side, base) in [(-1., 0), (1., 4)] {
            // Compatibility slots retain left/right stabilator, flap and aileron.
            p[base] = Panel::new(
                "STABILATOR",
                DVec3::new(side * s.span * 0.20, 0.02, z),
                DVec3::Y,
                3.3,
                1.5,
                0.48,
                0.,
                side * 0.18,
                0.,
                0.,
            );
            p[base + 1] = Panel::new(
                "TRAILING FLAP",
                DVec3::new(side * wing_x * 0.65, -0.1, wing_z),
                DVec3::Y,
                2.5,
                1.25,
                0.,
                0.,
                0.,
                -0.48,
                0.,
            );
            p[base + 2] = Panel::new(
                "AILERON",
                DVec3::new(side * wing_x * 1.35, -0.1, wing_z + 0.5),
                DVec3::Y,
                1.4,
                0.80,
                0.22,
                0.,
                -side * 0.65,
                -0.15,
                0.,
            );
            p[base + 3] = Panel::new(
                "RUDDER",
                DVec3::new(side * 1.25, 1.7, z - 0.4),
                DVec3::X,
                1.8,
                0.95,
                0.,
                -0.60,
                0.,
                0.,
                0.,
            );
            p[9 + if side < 0. { 0 } else { 1 }] = Panel::new(
                "LEADING FLAP",
                DVec3::new(side * wing_x, -0.08, wing_z - 1.6),
                DVec3::Y,
                1.8,
                1.,
                0.,
                0.,
                0.,
                -0.30,
                0.,
            );
        }
        p[8] = Panel::new(
            "DORSAL BRAKE",
            DVec3::new(0., 0.65, s.length * 0.17),
            DVec3::Y,
            2.0,
            0.45,
            0.,
            0.,
            0.,
            0.,
            -0.85,
        );
        if self == Self::F22 {
            p[8].area = 0.;
        } // Raptor uses differential surfaces, not a dorsal board.
        if self == Self::F35B {
            p[8].area = 0.;
        }
        if matches!(self, Self::F35B | Self::F22 | Self::Hornet) {
            p[8].area = 0.;
            p[2].brake = -0.35;
            p[6].brake = -0.35;
            p[3].brake = 0.28;
            p[7].brake = -0.28;
        }
        if self == Self::Hornet {
            for (side, i) in [(-1., 11), (1., 12)] {
                p[i] = Panel::new(
                    "LEX SPOILER",
                    DVec3::new(side * 1.12, 0.22, -s.length * 0.16),
                    DVec3::Y,
                    0.35,
                    0.45,
                    0.,
                    0.,
                    0.,
                    0.,
                    -0.9,
                );
            }
        }
        for panel in &mut p {
            panel.area *= s.area / 46.5;
        }
        let cant: f64 = if self == Self::Su27 { 0.06 } else { 0.34 };
        for (side, i) in [(-1., 3), (1., 7)] {
            p[i].normal = DVec3::new(cant.cos(), -side * cant.sin(), 0.);
            p[i].center.x = side * (if self == Self::Su27 { 1.6 } else { 1.18 } + cant.sin() * 1.5);
        }
        p
    }
}
#[derive(Clone, Copy)]
pub struct Spec {
    pub length: f64,
    pub span: f64,
    pub area: f64,
    pub empty: f64,
    pub fuel: f64,
    pub dry: f64,
    pub wet: f64,
    pub engines: usize,
    pub inertia: DVec3,
    pub g: f64,
    pub alpha: f64,
    pub limit_mach: f64,
    pub gun: u16,
    pub gun_interval: f64,
    pub ammo: [u8; 2],
}
#[derive(Clone, Copy, Debug)]
pub struct Panel {
    pub outline: Option<[DVec3;4]>,
    pub name: &'static str,
    pub center: DVec3,
    pub normal: DVec3,
    pub area: f64,
    pub chord: f64,
    pub pitch: f64,
    pub yaw: f64,
    pub roll: f64,
    pub flap: f64,
    pub brake: f64,
}
impl Panel {
    const fn empty() -> Self {
        Self {
            outline: None,
            name: "UNUSED",
            center: DVec3::ZERO,
            normal: DVec3::Y,
            area: 0.,
            chord: 0.,
            pitch: 0.,
            yaw: 0.,
            roll: 0.,
            flap: 0.,
            brake: 0.,
        }
    }
    fn new(
        name: &'static str,
        center: DVec3,
        normal: DVec3,
        area: f64,
        chord: f64,
        pitch: f64,
        yaw: f64,
        roll: f64,
        flap: f64,
        brake: f64,
    ) -> Self {
        Self {
            outline: None,
            name,
            center,
            normal,
            area,
            chord,
            pitch,
            yaw,
            roll,
            flap,
            brake,
        }
    }
    pub fn angle(self, actuator: DVec3, flaps: f64, brake: bool) -> f64 {
        (self.pitch * actuator.x
            + self.yaw * actuator.y
            + self.roll * actuator.z
            + self.flap * flaps
            + self.brake * if brake { 1. } else { 0. })
        .clamp(-0.9, 0.9)
    }
    pub fn axis(self) -> DVec3 {
        if let Some(p)=self.outline {
            if self.name!="STABILATOR" {
                let d=if self.name=="LEADING FLAP"{p[2]-p[3]}else{p[1]-p[0]};
                let d=d.normalize();return if self.normal.x.abs()>0.5{if d.y<0.{-d}else{d}}else if d.x<0.{-d}else{d};
            }
        }
        if self.normal.x.abs() > 0.5 {
            DVec3::Z.cross(self.normal).normalize()
        } else {
            DVec3::X
        }
    }
}
#[derive(Clone)]
pub struct Systems {
    pub raptor: raptor::flight_model::Dynamics,
    /// Arresting hook command; Hornet dynamics integrate its physical angle.
    pub hook_down: bool,
    pub hornet_canopy_target: bool,
    pub hornet_canopy: f64,
    pub hornet_wing_target: bool,
    pub hornet_wing_fold: f64,
    pub hornet_nav_lights: bool,
    pub hornet_strobe_lights: bool,
    pub hornet_landing_light: bool,
    pub external_stores: bool,
    pub engine_on: [bool; 2],
    pub manual_bays: bool,
    pub conversion: f64,
    pub conversion_target: bool,
    pub nozzle: [f64; 2],
    pub engines: [f64; 2],
    pub lift_fan: f64,
    pub stores: [bool; 10],
    pub bays: f64,
    pub bays_target: bool,
    pub side_bays: [bays::State; 2],
    pub side_request: Option<usize>,
    pub main_request: Option<usize>,
    pub main_stroke: [f64;6],
}
impl Default for Systems {
    fn default() -> Self {
        Self {
            raptor: default(),
            hook_down: false,
            hornet_canopy_target: false,
            hornet_canopy: 0.,
            hornet_wing_target: false,
            hornet_wing_fold: 0.,
            hornet_nav_lights: true,
            hornet_strobe_lights: true,
            hornet_landing_light: true,
            external_stores: false,
            engine_on: [true; 2],
            manual_bays: false,
            conversion: 0.,
            conversion_target: false,
            nozzle: [0.; 2],
            engines: [1.; 2],
            lift_fan: 1.,
            stores: [true; 10],
            bays: 0.,
            bays_target: false,
            side_bays: [bays::State::default(); 2],
            side_request: None,
            main_request: None,
            main_stroke: [0.;6],
        }
    }
}
pub fn select(f: &mut Flight, aircraft: Aircraft) {
    if f.carrier.active && aircraft!=Aircraft::Hornet {f.status="CARRIER MODE REQUIRES F/A-18F".into();return;}
    for side in 0..2 {
        if f.damage.wing_broken[side] {
            f.damage.wings[side] = 1.;
            for i in if side == 0 { [1, 2, 9] } else { [5, 6, 10] } {
                f.damage.surface_state[i] = crate::combat::damage::SurfaceState::Operational;
                f.damage.surfaces[i] = 1.;
            }
        }
    }
    f.aircraft = aircraft;
    f.damage.aircraft = aircraft;
    f.damage.wing_broken = [false; 2];
    f.damage.wing_stress = [0.; 2];
    f.systems = Systems::default();
    f.structure = Default::default();
    f.hornet = Default::default();
    if aircraft==Aircraft::Hornet {f.autopilot=false;}
    if f.vehicle == Vehicle::Spacecraft {
        return;
    }
    if aircraft != Aircraft::Gripen {
        f.fuel = aircraft.spec().fuel
            * if aircraft == Aircraft::F35B {
                0.45
            } else {
                0.60
            };
        let n = (aircraft.spec().ammo[0] + aircraft.spec().ammo[1]) as usize;
        f.systems.stores = std::array::from_fn(|i| i < n);
        if f.landed {
            let h = crate::collision::wheel_clearance(f);
            f.position = f.up()
                * (RADIUS + crate::physics::terrain_height(f.position.x, f.position.z).max(0.) + h);
        }
        if !f.landed {
            f.velocity *= 1.25;
        }
        f.airspeed = (f.velocity - f.wind).length();
        f.mach = f.airspeed / crate::physics::atmosphere(f.altitude()).3;
        f.thrust = if f.engine {
            jet_output(aircraft, f.spool, f.density, f.mach).0
        } else {
            0.
        };
    }
    if aircraft==Aircraft::Hornet&&!f.landed {
        let gamma=(f.vertical_speed()/f.airspeed.max(1.)).clamp(-0.3,0.3).asin();
        if let Some(trim)=hornet::trim::solve(f,f.altitude(),f.airspeed,gamma,0.) {
            f.orientation*=bevy::math::DQuat::from_rotation_x(trim.alpha+gamma-f.pitch());
            hornet::trim::apply(f,trim);
        }
    }
    f.status = format!(
        "{} / T FBW / F6 HANGAR{}",
        aircraft.name(),
        if aircraft == Aircraft::F35B {
            " / Y STOVL"
        } else {
            ""
        }
    );
}
pub fn angles(f: &Flight) -> [f64; SURFACES] {
    if f.aircraft == Aircraft::Hornet && f.hornet.initialized {return f.hornet.angles;}
    if f.aircraft == Aircraft::F22 && f.systems.raptor.actuators_initialized {
        return std::array::from_fn(|i| crate::control_surfaces::effective_angle(&f.damage, i, f.systems.raptor.angles[i]));
    }
    if f.aircraft == Aircraft::Gripen {
        crate::control_surfaces::angles(f.surface, f.flaps, f.airbrake)
    } else {
        let p = f.aircraft.panels();
        std::array::from_fn(|i| panel_angle(f.aircraft,i,p[i],f.surface,f.flaps,f.airbrake,f.alpha))
    }
}
pub fn surface_points(a: Aircraft) -> [DVec3; SURFACES] {
    if a == Aircraft::Gripen {
        std::array::from_fn(|i| crate::combat::damage::SURFACE_POINTS[i.min(8)])
    } else {
        a.panels().map(|p| p.center)
    }
}
pub fn surface_rotation(a:Aircraft,i:usize,angle:f64)->bevy::math::DQuat {
    let p=a.panels()[i];let sign=if a==Aircraft::F22&&p.normal.x.abs()<0.5{-1.}else{1.};
    bevy::math::DQuat::from_axis_angle(p.axis(),angle*sign)
}
fn panel_angle(a:Aircraft,i:usize,p:Panel,actuator:DVec3,flaps:f64,brake:bool,alpha:f64)->f64 {
    let flaps=if a==Aircraft::F22&&(i==9||i==10){flaps.max(((alpha.abs()-0.03)*1.6).clamp(0.,1.))}else{flaps};
    let angle = p.angle(actuator,flaps,brake);
    if a == Aircraft::F22 { raptor::flight_model::limit_angle(i, angle) } else { angle }
}
pub fn count(a: Aircraft) -> usize {
    if a == Aircraft::Gripen {
        9
    } else if a == Aircraft::Hornet {
        13
    } else {
        11
    }
}
pub fn attachment(a:Aircraft,d:&AircraftDamage,loads:&crate::control_surfaces::Loads)->f64 {
    if a!=Aircraft::F22{return loads.surfaces.iter().take(count(a)).map(|p|p.attached).fold(1.,f64::min);}
    let panels=a.panels();
    loads.surfaces.iter().take(count(a)).enumerate()
        .filter(|(i,_)|panels[*i].area>0.&&d.surface_state[*i]!=crate::combat::damage::SurfaceState::Detached)
        .map(|(_,p)|p.attached).fold(1.,f64::min)
}
pub fn stores_mass(f: &Flight) -> f64 {
    if f.aircraft == Aircraft::Gripen {
        0.
    } else {
        f.systems.stores.iter().enumerate().filter(|(_,s)|**s).map(|(i,_)|store_unit_mass(f.aircraft,i)).sum::<f64>()
            +if f.aircraft==Aircraft::F22&&f.systems.external_stores{70.}else{0.}
            +if f.aircraft==Aircraft::Hornet{hornet::stores::mass(f)}else{0.}
    }
}
fn store_unit_mass(a:Aircraft,i:usize)->f64{if matches!(a,Aircraft::F22|Aircraft::Hornet){if i<2{84.37}else{152.}}else{160.}}
pub fn jet_output(a: Aircraft, power: f64, rho: f64, mach: f64) -> (f64, f64) {
    if a == Aircraft::F22 { return raptor::flight_model::engine(power, rho, mach); }
    let s = a.spec();
    let lapse = (rho / 1.225).max(0.).powf(0.62) * (1. + 0.32 * mach * mach)
        / (1. + 0.16 * mach * mach * mach);
    let (augmentation, recovery) = (1., 1.);
    let wet = (s.wet - s.dry) * ((power - 0.88) / 0.12).clamp(0., 1.) * lapse
        * augmentation * recovery;
    ((s.dry * (power / 0.88).clamp(0., 1.)) * lapse * recovery + wet, wet)
}
pub fn mass_center(f: &Flight) -> DVec3 {
    if f.aircraft == Aircraft::F22 { return raptor::flight_model::center(f); }
    if f.aircraft == Aircraft::Gripen {
        return DVec3::ZERO;
    }
    let mut m = DVec3::new(0., -0.12, 0.15) * f.fuel;
    for (i, on) in f.systems.stores.iter().enumerate() {
        if *on {
            m += station_for(f,i) * store_unit_mass(f.aircraft,i);
        }
    }
    if f.aircraft==Aircraft::F22&&f.systems.external_stores{m+=DVec3::new(0.,-0.34,0.85)*70.;}
    m / f.mass()
}
pub fn station(a: Aircraft, i: usize) -> DVec3 {
    let s = a.spec();
    let side = if i % 2 == 0 { -1. } else { 1. };
    if a == Aircraft::Hornet {
        // Match the imported model's tip rails, outer and inner pylons,
        // then the two fuselage shoulder stations. Also used at release.
        let (span, height, aft) = match i / 2 {
            0 => (6.48, 0.12, 2.15),
            1 => (4.40, -0.48, 1.45),
            2 => (3.46, -0.55, 1.30),
            _ => (0.88, -0.60, 0.05),
        };
        return DVec3::new(side * span, height, aft);
    }
    if a == Aircraft::F22 {
        return if i < 2 {
            DVec3::new(side * 1.72, -0.40, -0.6)
        } else {
            main_bay::station(i-2)
        };
    }
    if a == Aircraft::F35B {
        return if i < 2 {
            DVec3::new(side * s.span * 0.45, -0.38, 1.5)
        } else {
            DVec3::new(((i - 2) as f64 - 1.5) * 0.4, -0.5, 0.25)
        };
    }
    DVec3::new(
        side * (1.4 + (i / 2) as f64 * s.span * 0.065),
        -0.65,
        (i / 2) as f64 * 0.16,
    )
}
pub fn store_rotation(f:&Flight,i:usize)->bevy::math::DQuat {
    if f.aircraft==Aircraft::Hornet&&i<2 {bevy::math::DQuat::from_rotation_z(if i==0{-1.}else{1.}*f.systems.hornet_wing_fold*1.3)}else{bevy::math::DQuat::IDENTITY}
}
pub fn station_for(f:&Flight,i:usize)->DVec3 {
    if f.aircraft==Aircraft::Hornet&&i<2 {
        // Imported WingTipPivot translations (0,0,+/-45), transformed by
        // .1 scale, +90deg yaw and the rig's (0,.35,2.56) offset.
        let pivot=DVec3::new(if i==0{-4.5}else{4.5},0.35,2.56);
        return pivot+store_rotation(f,i)*(station(f.aircraft,i)-pivot);
    }
    if f.aircraft==Aircraft::F22 && f.systems.external_stores && i>=6 {
        DVec3::new(if i%2==0{-3.35}else{3.35},-0.68,0.85)
    }else{station(f.aircraft,i)}
}
pub fn store_drag(f:&Flight)->f64 {
    if f.aircraft==Aircraft::F22 {
        let external=if f.systems.external_stores{f.systems.stores[6..8].iter().filter(|on|**on).count()}else{0};
        external as f64*0.0025+if f.systems.external_stores{0.0012}else{0.}+f.systems.bays*0.008
            + f.systems.side_bays.iter().map(|b| 0.0018*b.door+0.0009*b.launcher).sum::<f64>()
    }else if f.aircraft==Aircraft::Hornet {f.systems.stores.iter().filter(|on|**on).count() as f64*0.0018+f.hornet.tanks.fuel.iter().flatten().count() as f64*hornet::stores::DRAG}
    else{stores_mass(f)/160.*0.0018}
}
pub fn launch_station(f: &Flight, kind: crate::combat::Weapon, ammo: [u8; 2]) -> DVec3 {
    let s = f.aircraft.spec();
    let i = if kind == crate::combat::Weapon::Heat {
        s.ammo[0].saturating_sub(ammo[0]) as usize
    } else {
        (s.ammo[0] + s.ammo[1].saturating_sub(ammo[1])) as usize
    };
    if f.aircraft==Aircraft::F22 && kind==crate::combat::Weapon::Heat {
        bays::missile_pose(i.min(1), f.systems.side_bays[i.min(1)].launcher).translation.as_dvec3()
    } else if f.aircraft==Aircraft::F22 && kind==crate::combat::Weapon::Radar && !(f.systems.external_stores&&i>=6) {
        main_bay::store_position(i.saturating_sub(2).min(5),f.systems.main_stroke[i.saturating_sub(2).min(5)])
    } else {station_for(f, i.min(9))}
}
pub fn gun_origin(a: Aircraft) -> DVec3 {
    let l = a.spec().length;
    match a {
        Aircraft::Gripen => DVec3::new(0.4, -0.1, -6.),
        Aircraft::Su27 => DVec3::new(-0.88, 0.10, -l * 0.25),
        Aircraft::F35B => DVec3::new(0., -0.85, -0.6),
        Aircraft::F22 => DVec3::new(0.9, 0.30, -l * 0.18),
        Aircraft::Hornet => DVec3::new(0., 0.25, -l * 0.42),
    }
}
pub fn gun_name(a: Aircraft) -> &'static str {
    match a {
        Aircraft::Su27 => "GSH-30-1 / 30MM",
        Aircraft::F35B => "GAU-22/A POD / 25MM",
        Aircraft::F22 | Aircraft::Hornet => "M61A2 / 20MM",
        Aircraft::Gripen => "BK-27 / 27MM",
    }
}
pub fn missile_name(a: Aircraft, kind: crate::combat::Weapon) -> &'static str {
    match (a, kind) {
        (Aircraft::Su27, crate::combat::Weapon::Heat) => "R-73",
        (Aircraft::Su27, crate::combat::Weapon::Radar) => "R-77",
        (Aircraft::Gripen, crate::combat::Weapon::Heat) => "IRIS-T",
        (Aircraft::Gripen, crate::combat::Weapon::Radar) => "METEOR",
        (Aircraft::F22 | Aircraft::Hornet, crate::combat::Weapon::Heat) => "AIM-9X",
        (Aircraft::F22, crate::combat::Weapon::Radar) => "AIM-120D",
        (_, crate::combat::Weapon::Heat) => "IR MISSILE",
        _ => "AIM-120",
    }
}
pub fn panel_loads(
    a: Aircraft,
    air: DVec3,
    rates: DVec3,
    rho: f64,
    mach: f64,
    actuator: DVec3,
    flaps: f64,
    brake: bool,
    d: &AircraftDamage,
) -> crate::control_surfaces::Loads {
    panel_loads_angles(a,air,rates,rho,mach,actuator,flaps,brake,d,None,None)
}
/// Evaluate a command against a frozen elastic shape. FBW may call this many
/// times per step; it must never advance the structural state itself.
pub fn panel_loads_structural(
    a: Aircraft, air: DVec3, rates: DVec3, rho: f64, mach: f64,
    actuator: DVec3, flaps: f64, brake: bool, d: &AircraftDamage,
    structure: &crate::structural::State,
) -> crate::control_surfaces::Loads {
    panel_loads_angles(a,air,rates,rho,mach,actuator,flaps,brake,d,None,Some(structure))
}
pub fn actual_panel_loads(f: &Flight, air: DVec3, rho: f64) -> crate::control_surfaces::Loads {
    panel_loads_angles(f.aircraft, air, f.rates, rho, f.mach, f.surface, f.flaps, f.airbrake, &f.damage,
        Some(&f.systems.raptor.angles),Some(&f.structure))
}
pub fn raptor_structural_surface(i: usize) -> Option<usize> {
    use crate::structural::{LEFT_FIN, LEFT_TAIL, LEFT_WING, RIGHT_FIN, RIGHT_TAIL, RIGHT_WING};
    match i {
        0 => Some(LEFT_TAIL), 4 => Some(RIGHT_TAIL),
        1 | 2 | 9 => Some(LEFT_WING), 5 | 6 | 10 => Some(RIGHT_WING),
        3 => Some(LEFT_FIN), 7 => Some(RIGHT_FIN), _ => None,
    }
}
/// Some legacy aerodynamic panel centers coincide with the hinge, even though
/// the resultant pressure acts over the panel. Sample its elastic response
/// at least 45% of the estimated chord from the hinge, without changing the
/// legacy rigid-body force application point.
pub fn raptor_panel_flex_sample(i: usize, point: DVec3) -> DVec3 {
    let Some(c) = crate::structural::f22_config().panels.get(i).copied().flatten() else {return point;};
    point + c.span * ((0.45 - c.span_fraction(point)).max(0.) * c.length)
}
fn panel_loads_angles(
    a: Aircraft, air: DVec3, rates: DVec3, rho: f64, mach: f64,
    actuator: DVec3, flaps: f64, brake: bool, d: &AircraftDamage,
    actual: Option<&[f64; SURFACES]>, structure: Option<&crate::structural::State>,
) -> crate::control_surfaces::Loads {
    let mut result = crate::control_surfaces::Loads::default();
    let config = if a == Aircraft::F22 && structure.is_some() {Some(crate::structural::f22_config())} else {None};
    for (i, p) in a.panels().into_iter().enumerate() {
        if p.area == 0. || d.surface_state[i] == crate::combat::damage::SurfaceState::Detached {
            continue;
        }
        let structural_surface = if a == Aircraft::F22 {raptor_structural_surface(i)} else {None};
        let angle = crate::control_surfaces::effective_angle(d, i, actual.map_or_else(
            || panel_angle(a,i,p,actuator,flaps,brake,(-air.y).atan2(-air.z)), |v| v[i]));
        let hinge = if a == Aircraft::F22 {surface_rotation(a,i,angle)} else {bevy::math::DQuat::IDENTITY};
        let elastic = structure.zip(structural_surface).zip(config);
        let panel_sample = if elastic.is_some() {raptor_panel_flex_sample(i,p.center)} else {p.center};
        let frame = elastic.map_or(bevy::math::DQuat::IDENTITY, |((state, surface),config)| state.frame_rotation(config,surface,p.center));
        let panel_frame = elastic.map_or(bevy::math::DQuat::IDENTITY, |((state,_),config)| state.panel_frame_rotation(config,i,panel_sample));
        let center = elastic.map_or(p.center, |((state,surface),config)| {
            let parent = state.deform_point(config,surface,p.center);
            parent + frame * (hinge * (state.deform_panel_point(config,i,panel_sample)-panel_sample))
        });
        let base_normal = frame * (panel_frame * p.normal);
        let elastic_velocity = elastic.map_or(DVec3::ZERO, |((state,surface),config)| {
            state.point_velocity(config,surface,p.center)
                + frame * (hinge * state.panel_point_velocity(config,i,panel_sample))
        });
        let v = air + rates.cross(center) + elastic_velocity;
        let speed = v.length();
        if speed < 0.01 {
            continue;
        }
        let flow = v / speed;
        let incidence = (-v.dot(base_normal)).atan2(-v.z);
        let polar = |x: f64| {
            let attached = (1. - ((x.abs() - 0.30) / 0.6).clamp(0., 1.)).powi(2);
            ((attached * 2.4 + (1. - attached) * 0.85) * (2. * x).sin()) / (1. + 0.12 * mach * mach)
        };
        let cl = polar(incidence - angle) - polar(incidence);
        let lift = (base_normal - flow * base_normal.dot(flow)).normalize_or_zero();
        let lift_delta=if a==Aircraft::F22 {
            let rotation=frame*hinge*frame.conjugate();let normal=rotation*base_normal;
            let mut chord=p.normal.cross(p.axis()).normalize_or_zero();if chord.z>0.{chord=-chord;}let chord=frame*(panel_frame*chord);
            let before=(-v.dot(base_normal)).atan2(v.dot(chord));
            let after=(-v.dot(normal)).atan2(v.dot(rotation*chord));
            let rotated=(normal-flow*normal.dot(flow)).normalize_or_zero();
            rotated*polar(after)-lift*polar(before)
        }else{lift*cl};
        let cd = if p.brake != 0. {
            1.8 * angle.sin().powi(2)
        } else if a==Aircraft::F22 {
            0.10 * angle.sin().powi(2) + 0.08 * lift_delta.length_squared()
        } else {
            0.10 * angle.sin().powi(2) + 0.08 * cl * cl
        };
        let health = if matches!(
            d.surface_state[i],
            crate::combat::damage::SurfaceState::Jammed(_)
        ) {
            1.
        } else {
            d.surfaces[i]
        };
        let effectiveness = if a==Aircraft::F22 {
            match i {0|4=>raptor::STABILATOR_EFFECTIVENESS,2|6=>raptor::AILERON_EFFECTIVENESS,_=>1.}
        }else{1.};
        let force = (lift_delta - flow * cd) * (0.5 * rho * speed * speed * p.area * health * effectiveness);
        let load = &mut result.surfaces[i];
        load.force = force;
        load.moment = center.cross(force);
        load.deflection = angle;
        load.attached = (1. - incidence.abs() / 1.4).clamp(0., 1.);
        result.force += force;
        result.moment += load.moment;
    }
    result
}
/// Engine result in body axes, including force application offsets. No direct
/// orientation changes are used for vectoring or hover control.
pub fn propulsion(f: &mut Flight, dt: f64, rho: f64, mach: f64) -> (DVec3, DVec3, f64) {
    let s = f.aircraft.spec();
    if f.aircraft == Aircraft::Hornet && f.damage.hydraulics > 0.12 {
        let canopy = if f.systems.hornet_canopy_target { 1. } else { 0. };
        let wing = if f.systems.hornet_wing_target { 1. } else { 0. };
        f.systems.hornet_canopy += (canopy-f.systems.hornet_canopy).clamp(-dt/3.2,dt/3.2);
        f.systems.hornet_wing_fold += (wing-f.systems.hornet_wing_fold).clamp(-dt/5.,dt/5.);
    }
    if f.aircraft == Aircraft::F22 {
        let powered=f.damage.hydraulics>0.12 && !f.crashed;
        main_bay::advance(&mut f.systems,powered,dt);
        for i in 0..2 {
            f.systems.side_bays[i].advance(f.systems.manual_bays || f.systems.side_request==Some(i), powered, dt);
        }
    }
    let t = f.systems.conversion_target && f.aircraft == Aircraft::F35B;
    if f.aircraft!=Aircraft::F22 && f.damage.hydraulics > 0.12 {
        let target = if f.systems.bays_target { 1. } else { 0. };
        f.systems.bays += (target - f.systems.bays).clamp(-dt * 2.5, dt * 2.5);
    }
    let can_convert = f.airspeed < 150. && f.systems.lift_fan > 0.15;
    let target = if t && can_convert { 1. } else { 0. };
    if f.damage.hydraulics > 0.12 {
        f.systems.conversion += (target - f.systems.conversion).clamp(-dt / 3., dt / 3.);
    }
    let conv = f.systems.conversion;
    let lapse = (rho / 1.225).max(0.).powf(0.62) * (1. + 0.32 * mach * mach)
        / (1. + 0.16 * mach * mach * mach);
    let dry = s.dry * (f.spool / 0.88).clamp(0., 1.);
    let burner = (s.wet - s.dry) * ((f.spool - 0.88) / 0.12).clamp(0., 1.) * (1. - conv);
    let installed = if f.aircraft == Aircraft::F22 {
        raptor::flight_model::installed_at(f, rho, mach).0
    } else { ((dry + burner) * (1. - conv) + s.wet * f.spool * conv) * lapse };
    let running = f.engine && (f.fuel > 0. || f.infinite_fuel);
    let mut force = DVec3::ZERO;
    let mut moment = DVec3::ZERO;
    for (i, pos) in f
        .aircraft
        .engine_positions()
        .iter()
        .enumerate()
        .take(s.engines)
    {
        let desired = if f.aircraft == Aircraft::F22 {
            raptor::nozzle_target(i, raptor::flight_model::nozzle_command(f))
        } else if f.aircraft == Aircraft::F35B {
            conv * std::f64::consts::FRAC_PI_2
        } else {
            0.
        };
        if f.damage.nozzles[i] > 0.1 && f.damage.hydraulics > 0.12 {
            f.systems.nozzle[i] += (desired - f.systems.nozzle[i]).clamp(-dt, dt);
        }
        let power = installed / s.engines as f64
            * f.systems.engines[i]
            * if running && f.systems.engine_on[i] {
                1.
            } else {
                0.
            };
        let dir = DVec3::new(0., f.systems.nozzle[i].sin(), -f.systems.nozzle[i].cos());
        let mut thrust = dir * power;
        let fan_share = pos.z / (pos.z + 2.6);
        if f.aircraft == Aircraft::F35B {
            thrust *= 1. - conv * (fan_share + (1. - fan_share) * 0.10);
        }
        force += thrust;
        moment += pos.cross(thrust);
        if f.aircraft == Aircraft::F35B {
            let fan = DVec3::Y * power * conv * fan_share * 0.90 * f.systems.lift_fan;
            force += fan;
            moment += DVec3::new(0., 0., -2.6).cross(fan);
            // Reaction control/roll posts use a finite share of available lift.
            let mut demand = if f.assist {
                DVec3::Y.cross(f.orientation.conjugate() * f.up()) * 2.5
                    + (f.controls * DVec3::new(0.55, 0.35, -0.8) - f.rates) * 1.8
            } else {
                f.controls * DVec3::new(0.6, 0.35, -0.65)
            };
            if f.assist {
                let cg = mass_center(f);
                demand.x -= cg.z / (0.06 * (pos.z + 2.6));
                demand.z += cg.x / (2. * s.span * 0.33 * 0.045);
            }
            let demand = demand.clamp(DVec3::splat(-1.), DVec3::ONE) * f.damage.hydraulics;
            // Pitch control transfers lift between fan and aft nozzle; roll
            // posts receive 10% of total vertical thrust. Differential fore/aft
            // exhaust at the posts supplies a bounded yaw couple.
            let transfer = demand.x * power * conv * 0.06;
            moment += DVec3::new(0., 0., -2.6).cross(DVec3::Y * transfer)
                + pos.cross(-DVec3::Y * transfer);
            let arm = s.span * 0.33;
            for side in [-1., 1.] {
                let post = DVec3::new(side * arm, 0., 0.);
                let thrust = DVec3::new(
                    0.,
                    power * conv * (0.05 + side * demand.z * 0.045),
                    -side * demand.y * power * conv * 0.02,
                );
                force += thrust;
                moment += post.cross(thrust);
            }
        }
    }
    (force, moment, burner)
}

pub fn study() {
    let mut rows = Vec::new();
    for a in Aircraft::ALL {
        let mut f = crate::combat::flight();
        select(&mut f, a);
        f.infinite_fuel = true;
        let mut peak_g = 0_f64;
        let mut peak_alpha = 0_f64;
        for i in 0..1200 {
            f.controls = if i < 600 {
                DVec3::new(0.25, 0., 0.15)
            } else {
                DVec3::ZERO
            };
            crate::physics::simulate(&mut f, &Environment::default(), 1. / 120.);
            peak_g = peak_g.max(f.g_load.abs());
            peak_alpha = peak_alpha.max(f.alpha.abs());
        }
        rows.push(serde_json::json!({"aircraft":a.slug(),"speed":f.airspeed,"altitude":f.altitude(),"peak_g":peak_g,"peak_alpha_deg":peak_alpha.to_degrees(),"crashed":f.crashed,"finite":f.position.is_finite()&&f.rates.is_finite(),"mass":f.mass(),"fuel":f.fuel}));
    }
    let (cobra, peak, samples) = cobra_trial();
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"flight":rows,"cobra_peak_alpha_deg":peak.to_degrees(),"cobra_final_alpha_deg":cobra.alpha.to_degrees(),"cobra_speed":cobra.airspeed,"cobra_crashed":cobra.crashed,"cobra_samples":samples,"note":"Cobra experiment uses a pull, unload and small recovery corrections; no attitude animation."})).unwrap());
}

fn cobra_trial() -> (Flight, f64, Vec<serde_json::Value>) {
    let mut cobra = crate::combat::flight();
    select(&mut cobra, Aircraft::Su27);
    cobra.position = surface_point(0., 0., 6000.);
    cobra.velocity = DVec3::new(0., 0., -140.);
    cobra.assist = false;
    cobra.gear = false;
    cobra.gear_extension = 0.;
    cobra.controls = DVec3::X;
    cobra.throttle = 0.8;
    let mut peak = 0_f64;
    let mut samples = Vec::new();
    for i in 0..1440 {
        if peak > 85_f64.to_radians() || i > 240 {
            // Unload while the tail is blanketed, then make ordinary small
            // stick corrections once conventional flow returns.
            cobra.controls.x = if cobra.alpha.abs() > 0.55 {
                0.
            } else {
                ((0.08 - cobra.alpha) * 0.6 - cobra.rates.x * 0.45).clamp(-0.5, 0.5)
            };
        }
        crate::physics::simulate(&mut cobra, &Environment::default(), 1. / 120.);
        peak = peak.max(cobra.alpha);
        if i % 60 == 0 {
            samples.push(serde_json::json!({"t":i as f64/120.,"alpha":cobra.alpha.to_degrees(),"rate":cobra.rates.x.to_degrees(),"surface":cobra.surface.x,"pitch_coefficient":cobra.aero_coeff.pitch}));
        }
    }
    (cobra, peak, samples)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::damage::SurfaceState;
    #[test]
    fn su27_can_pitch_past_ninety_degrees_and_recover_using_aerodynamic_moments() {
        let (f, peak, samples) = cobra_trial();
        assert!(
            peak > 90_f64.to_radians() && peak < 140_f64.to_radians(),
            "peak alpha {}: {:?}",
            peak.to_degrees(),
            samples
        );
        assert!(
            f.alpha.abs() < 25_f64.to_radians(),
            "recovery alpha {}",
            f.alpha.to_degrees()
        );
        assert!(!f.crashed);
        assert!(f.rates.is_finite());
    }
    #[test]
    fn all_new_aircraft_have_independent_lateral_and_longitudinal_controls() {
        for a in [
            Aircraft::Su27,
            Aircraft::F35B,
            Aircraft::F22,
            Aircraft::Hornet,
        ] {
            let d = AircraftDamage {
                aircraft: a,
                ..default()
            };
            let air = DVec3::new(0., -4., -180.);
            let load = |u| panel_loads(a, air, DVec3::ZERO, 1., 0.5, u, 0., false, &d);
            assert!(
                load(DVec3::X).moment.x > load(-DVec3::X).moment.x,
                "{a:?} pitch"
            );
            assert!(
                load(DVec3::Y).moment.y > load(-DVec3::Y).moment.y,
                "{a:?} yaw"
            );
            assert!(
                (load(DVec3::Z).moment.z - load(-DVec3::Z).moment.z).abs() > 1000.,
                "{a:?} roll"
            );
        }
    }
    #[test]
    fn jam_and_detachment_use_the_same_surface_definition_as_rendering() {
        for a in [
            Aircraft::Su27,
            Aircraft::F35B,
            Aircraft::F22,
            Aircraft::Hornet,
        ] {
            for i in 0..count(a) {
                if a.panels()[i].area == 0. {
                    continue;
                }
                let mut d = AircraftDamage {
                    aircraft: a,
                    ..default()
                };
                d.surface_state[i] = SurfaceState::Jammed(0.23);
                let load = |d: &AircraftDamage, u| {
                    panel_loads(
                        a,
                        DVec3::new(0., -8., -160.),
                        DVec3::ZERO,
                        1.,
                        0.5,
                        u,
                        0.,
                        false,
                        d,
                    )
                    .surfaces[i]
                };
                assert_eq!(load(&d, DVec3::ONE).force, load(&d, -DVec3::ONE).force);
                assert_eq!(load(&d, DVec3::ZERO).deflection, 0.23);
                d.surface_state[i] = SurfaceState::Detached;
                assert_eq!(load(&d, DVec3::ONE).force, DVec3::ZERO);
            }
        }
    }
    #[test]
    fn engine_out_creates_a_yawing_moment_and_nozzle_failure_holds_angle() {
        let mut f = Flight::approach();
        select(&mut f, Aircraft::F22);
        f.spool = 1.;
        f.surface.x = 0.5;
        let (_, symmetric, _) = propulsion(&mut f, 0.1, 1.225, 0.2);
        assert!(symmetric.y.abs() < 1e-6);
        f.systems.engines[0] = 0.;
        let (_, asym, _) = propulsion(&mut f, 0.1, 1.225, 0.2);
        assert!(asym.y.abs() > 50000.);
        f.damage.nozzles[1] = 0.;
        let q = f.systems.nozzle[1];
        f.surface.x = -1.;
        propulsion(&mut f, 1., 1.225, 0.2);
        assert_eq!(f.systems.nozzle[1], q);
    }
    #[test]
    fn stovl_is_rate_limited_and_produces_balanced_vertical_thrust() {
        let mut f = Flight::approach();
        select(&mut f, Aircraft::F35B);
        f.airspeed = 0.;
        f.position = DVec3::Y * (RADIUS + 1000.);
        f.orientation = bevy::math::DQuat::IDENTITY;
        f.spool = 1.;
        f.controls = DVec3::ZERO;
        f.rates = DVec3::ZERO;
        f.systems.conversion_target = true;
        propulsion(&mut f, 0.1, 1.225, 0.);
        assert!(f.systems.conversion < 0.04);
        for _ in 0..400 {
            propulsion(&mut f, 0.01, 1.225, 0.);
        }
        let (force, moment, _) = propulsion(&mut f, 0.01, 1.225, 0.);
        assert!(force.y > f.mass() * 9.81);
        assert!(force.z.abs() < 1e-4);
        assert!((moment - mass_center(&f).cross(force)).x.abs() < 1e-5);
        f.systems.lift_fan = 0.;
        let (failed, _, _) = propulsion(&mut f, 0.01, 1.225, 0.);
        assert!(failed.y < force.y * 0.5);
    }
    #[test]
    fn new_airframes_have_larger_shaped_skins_and_detached_surfaces_leave_collision() {
        for a in [
            Aircraft::Su27,
            Aircraft::F35B,
            Aircraft::F22,
            Aircraft::Hornet,
        ] {
            let h = crate::airframe::aircraft_hulls(a);
            assert!(h.len() > 20);
            assert!(crate::airframe::distance_to_aircraft(DVec3::ZERO, a) < 0.01);
            assert!(crate::airframe::distance_to_aircraft(DVec3::new(100., 0., 0.), a) > 80.);
        }
    }
    #[test]
    fn full_six_dof_hover_is_controllable_and_lift_fan_failure_causes_descent() {
        let mut f = Flight::approach();
        select(&mut f, Aircraft::F35B);
        f.position = DVec3::Y * (RADIUS + 500.);
        f.orientation = bevy::math::DQuat::IDENTITY;
        f.velocity = DVec3::ZERO;
        f.rates = DVec3::ZERO;
        f.surface = DVec3::ZERO;
        f.controls = DVec3::ZERO;
        f.gear_extension = 1.;
        f.gear = true;
        f.systems.conversion = 1.;
        f.systems.conversion_target = true;
        f.systems.nozzle = [std::f64::consts::FRAC_PI_2; 2];
        f.flaps = 0.;
        f.spool = 0.97;
        f.throttle = 0.97;
        let e = Environment {
            wind_enabled: false,
            ..default()
        };
        let initial = f.altitude();
        for _ in 0..1200 {
            let rho = crate::physics::atmosphere(f.altitude()).2;
            let lapse = (rho / 1.225).powf(0.62);
            f.throttle = ((f.mass() * 9.81 - f.mass() * f.vertical_speed() * 0.6)
                / (f.aircraft.spec().wet * lapse))
                .clamp(0., 1.);
            crate::physics::simulate(&mut f, &e, 1. / 120.);
        }
        assert!(!f.crashed);
        assert!(
            (f.altitude() - initial).abs() < 20.,
            "hover drift {}",
            f.altitude() - initial
        );
        assert!(f.velocity.length() < 10., "hover velocity {:?}", f.velocity);
        f.damage.lift_fan = 0.;
        for _ in 0..240 {
            crate::physics::simulate(&mut f, &e, 1. / 120.);
        }
        assert!(f.vertical_speed() < -5.);
        assert!(f.position.is_finite());
    }
    #[test]
    fn scenario_reset_preserves_selection_and_space_is_separate() {
        let mut f = Flight::runway();
        select(&mut f, Aircraft::Su27);
        let mut v = crate::ViewState::default();
        for s in [0, 1, 2, 4, 5, 6, 7, 8, 3, 1] {
            crate::reset(&mut f, s, &mut v);
            assert_eq!(f.aircraft, Aircraft::Su27);
            if s == 3 {
                assert_eq!(f.vehicle, Vehicle::Spacecraft);
            } else {
                assert_eq!(f.damage.aircraft, Aircraft::Su27);
            }
        }
    }
    #[test]
    fn new_gear_stows_inside_its_bay_and_supports_each_aircraft_on_the_runway() {
        for a in [
            Aircraft::Su27,
            Aircraft::F35B,
            Aircraft::F22,
            Aircraft::Hornet,
        ] {
            let mut f = Flight::runway();
            select(&mut f, a);
            let clearance = crate::collision::wheel_clearance(&f);
            assert!(clearance > 1.9);
            f.gear_extension = 0.;
            for i in 0..3 {
                let (mount, wheel, _) = visual::gear_pose(&f, i, false);
                assert!(
                    wheel.y - 0.17 > -0.6,
                    "{a:?} folded wheel protrudes through door"
                );
                assert!(
                    (wheel.z - mount.z + 0.6).abs() + 0.34 < 1.25,
                    "{a:?} wheel outside bay length"
                );
            }
            f.gear_extension = 1.;
            f.throttle = 0.;
            f.brakes = true;
            for _ in 0..600 {
                crate::physics::simulate(&mut f, &Environment::default(), 1. / 120.);
            }
            assert!(!f.crashed, "{a:?} runway support: {}", f.status);
            assert!(f.gear_health.iter().all(|v| *v > 0.9));
            assert!(f.velocity.length() < 1.);
        }
    }
    #[test]
    fn fixed_wing_cutouts_and_hinged_surfaces_have_finite_solid_geometry() {
        for a in [
            Aircraft::Su27,
            Aircraft::F35B,
            Aircraft::F22,
            Aircraft::Hornet,
        ] {
            for side in [-1., 1.] {
                let pieces = visual::fixed_wing(a, side);
                assert!(!pieces.is_empty());
                for p in pieces {
                    assert!(p.len() >= 3);
                    assert!(p.iter().all(|v| v.is_finite()));
                }
            }
            for p in a.panels().iter().filter(|p| p.area > 0.) {
                assert!((p.normal.length() - 1.).abs() < 1e-10);
                assert!((p.axis().length() - 1.).abs() < 1e-10);
            }
        }
    }
}
