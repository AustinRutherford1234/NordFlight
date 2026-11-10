//! Camera-projected combat symbology and the cannon's constant-velocity solution.
use super::{CANNON_MUZZLE_SPEED, Combat, Target, Weapon};
use crate::{FlightCamera, ViewState, physics::*, world};
use bevy::{
    math::{DQuat, DVec3},
    prelude::*,
};

const GREEN: Color = Color::srgb(0.43, 1.0, 0.76);
const AMBER: Color = Color::srgb(1.0, 0.73, 0.30);
const RED: Color = Color::srgb(1.0, 0.36, 0.29);
const DIM: Color = Color::srgba(0.46, 0.78, 0.80, 0.58);

pub struct GunSolution {
    pub direction: DVec3,
    pub time: f64,
    pub range: f64,
    pub aligned: bool,
}
pub fn gun_solution(f: &Flight, target: &Target) -> Option<GunSolution> {
    if !target.alive{return None;}
    gun_intercept(f,target.position,target.velocity)
}
pub(crate) fn gun_intercept(f:&Flight,position:DVec3,velocity:DVec3)->Option<GunSolution>{
    let muzzle = f.position + f.orientation * crate::aircraft::gun_origin(f.aircraft);
    let r = position - muzzle;
    let v = velocity - f.velocity;
    let g = -f.up() * 9.81;
    if !r.is_finite()||!v.is_finite()||r.length() > 2000.0 || r.length() < 30.0 {
        return None;
    }
    let displacement = |t: f64| r + v * t - g * (0.5 * t * t);
    let error = |t: f64| displacement(t).length() - CANNON_MUZZLE_SPEED * t;
    // Find the earliest intercept within the actual cannon-round lifetime.
    let mut low = 0.0;
    let mut high = None;
    for i in 1..=100 {
        let t = i as f64 * 0.025;
        if error(t) <= 0.0 {
            high = Some(t);
            break;
        }
        low = t;
    }
    let mut high = high?;
    for _ in 0..30 {
        let mid = (low + high) * 0.5;
        if error(mid) > 0.0 {
            low = mid;
        } else {
            high = mid;
        }
    }
    let direction = displacement(high).normalize();
    Some(GunSolution {
        direction,
        time: high,
        range: r.length(),
        aligned: direction.dot(f.forward()) > (0.0025 + 3.0 / r.length()).cos(),
    })
}

/// Gun ranging is independent of missile lock; ignore allies and terrain-masked
/// aircraft. Prefer the designation, then the hostile nearest the firing axis.
pub fn gun_target<'a>(c:&'a Combat,f:&Flight)->Option<(usize,&'a Target)>{
    if !c.active{return None;}
    let usable=|i:usize,t:&Target|t.alive&&!c.bvr.as_ref().is_some_and(|b|b.friendly(i))
        && {let r=t.position-f.position;let range=r.length();
            (30. ..=2000.).contains(&range)&&r.dot(f.forward())>range*0.5}
        && super::guidance::terrain_visible(f.position,t.position);
    if let Some(t)=c.targets.get(c.selected).filter(|t|usable(c.selected,t)){return Some((c.selected,t));}
    c.targets.iter().enumerate().filter(|(i,t)|usable(*i,t)).max_by(|(_,a),(_,b)|{
        let alignment=|t:&Target|(t.position-f.position).normalize().dot(f.forward());
        alignment(a).total_cmp(&alignment(b))
    })
}

/// Keep the designated aircraft visible before it enters cannon range. When
/// the gun ranges another usable hostile, its diamond follows that same target.
pub fn cockpit_target<'a>(c:&'a Combat,f:&Flight)->Option<(usize,&'a Target)>{
    if !c.active||f.crashed{return None;}
    gun_target(c,f).or_else(||c.targets.get(c.selected).filter(|t|t.alive
        &&!c.bvr.as_ref().is_some_and(|b|b.friendly(c.selected))
        &&super::guidance::terrain_visible(f.position,t.position)).map(|t|(c.selected,t)))
}

/// A director cue: put `impact` over the target's CURRENT image. It represents
/// the miss of a centreline round fired now, relative to that target's predicted
/// position at time of flight, translated back to its current position. This
/// includes inherited aircraft velocity, gravity and constant target velocity.
pub struct GunPipper {
    pub bore: DVec3,
    pub impact: DVec3,
    pub range: f64,
    pub time: f64,
    pub aligned: bool,
}
pub fn gun_pipper(f: &Flight, target: &Target) -> Option<GunPipper> {
    if (target.position - f.position).dot(f.forward()) <= 0.0 { return None; }
    let solution = gun_solution(f, target)?;
    let muzzle = f.position + f.orientation * crate::aircraft::gun_origin(f.aircraft);
    let impact = muzzle
        + (f.velocity + f.forward() * CANNON_MUZZLE_SPEED - target.velocity) * solution.time
        - f.up() * (4.905 * solution.time * solution.time);
    Some(GunPipper {
        bore: muzzle + f.forward() * solution.range,
        impact,
        range: solution.range,
        time: solution.time,
        aligned: solution.aligned,
    })
}

pub fn closing(f: &Flight, target: &Target) -> f64 {
    (f.velocity - target.velocity).dot((target.position - f.position).normalize_or_zero())
}

/// LCOS-style virtual earlier shots, assuming the current body rates and
/// body-relative velocity stayed constant over each round's flight time.
/// Rail spacing is the supplied reference wingspan; the origin follows the
/// selected aircraft's real game muzzle. Includes a 30 m close-range anchor.
pub fn funnel(f: &Flight, span: f64) -> Vec<[DVec3; 2]> {
    let body_velocity = f.orientation.conjugate() * f.velocity;
    (0..=28)
        .map(|i| {
            let range = 30.0 + i as f64 * (900.0 - 30.0) / 28.0;
            let t = range / CANNON_MUZZLE_SPEED;
            let old_rotation = f.orientation * DQuat::from_scaled_axis(-f.rates * t);
            let mut old_position = f.position;
            for step in 0..8 {
                let age = (step as f64 + 0.5) * t / 8.0;
                old_position -= f.orientation
                    * DQuat::from_scaled_axis(-f.rates * age)
                    * body_velocity
                    * (t / 8.0);
            }
            let centre = old_position
                + old_rotation * crate::aircraft::gun_origin(f.aircraft)
                + (old_rotation * body_velocity + old_rotation * -DVec3::Z * CANNON_MUZZLE_SPEED)
                    * t
                - f.up() * (4.905 * t * t);
            let half = f.orientation * DVec3::X * (span * 0.5);
            [centre - half, centre + half]
        })
        .collect()
}

pub fn cockpit_lines(combat: &Combat, f: &Flight) -> Option<[String; 2]> {
    if !combat.active || f.vehicle != Vehicle::Gripen {
        return None;
    }
    let state = if combat.cue(f).is_err() {
        "NO LOCK"
    } else if combat.lock >= combat.weapon.lock_time() {
        "LOCK / SPACE"
    } else {
        "ACQUIRE"
    };
    let missile = if combat.weapon == Weapon::Heat && matches!(f.aircraft,crate::aircraft::Aircraft::F22|crate::aircraft::Aircraft::Hornet) {
        "AIM-9X"
    } else if combat.weapon == Weapon::Heat {
        "IR"
    } else if combat.bvr.is_some() {
        "LR"
    } else {
        "RAD"
    };
    let gun = combat.fight.as_ref().map_or(0, |fight| fight.gun);
    let (_, _, _, threat) = combat.threats(f);
    Some([
        format!(
            "{} {}  {}",
            missile,
            combat.ammo[combat.weapon.index()],
            state
        ),
        if let Some(kind) = threat {
            if kind == Weapon::Heat {
                "MISSILE / V COUNTERMEASURES".into()
            } else {
                "MISSILE / BKSP CHAFF".into()
            }
        } else {
            gun_target(combat,f)
                .and_then(|(_,t)| gun_solution(f, t))
                .map_or_else(
                    || format!("GUN {}  V/BKSP CM", gun),
                    |s| format!("GUN {}  TOF {:.2}S  U", gun, s.time),
                )
        },
    ])
}
pub fn lock_fraction(combat: &Combat) -> f32 {
    (combat.lock / combat.weapon.lock_time()).clamp(0.0, 1.0) as f32
}
pub fn weapon_display(combat: &Combat, f: &Flight) -> Option<Vec<String>> {
    if !combat.active || f.vehicle != Vehicle::Gripen {
        return None;
    }
    let t = combat.targets.get(combat.selected)?;
    Some(vec![
        if f.aircraft==crate::aircraft::Aircraft::Gripen{"WEAPONS / TARGET".into()}else{format!("{} / {}",crate::aircraft::missile_name(f.aircraft,combat.weapon),crate::aircraft::gun_name(f.aircraft))},
        format!(
            "IR {} RAD {} GUN {}",
            combat.ammo[0],
            combat.ammo[1],
            combat.fight.as_ref().map_or(0, |fight| fight.gun)
        ),
        format!(
            "T{:02} RNG {:.2}KM",
            combat.selected + 1,
            t.position.distance(f.position) / 1000.0
        ),
        format!(
            "Vc {:+.0} LOCK {:.0}%",
            closing(f, t),
            lock_fraction(combat) * 100.0
        ),
        format!(
            "HULL {:.0}% ENG {:.0}%",
            t.health * 100.0,
            t.damage.engine * 100.0
        ),
        "SPACE MISSILE / U GUN".into(),
        "V BOTH / BKSP CHAFF".into(),
    ])
}

#[derive(Component)]
pub struct Stroke(usize);
#[derive(Component)]
pub struct Caption(usize);

pub fn setup(mut c: Commands) {
    // Fixed pool: no spawning or asset allocation in the per-frame draw system.
    for i in 0..540 {
        c.spawn((
            Stroke(i),
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                ..default()
            },
            UiTransform::default(),
            BackgroundColor(GREEN),
            ZIndex(if i % 2 == 0 { 25 } else { 26 }),
        ));
    }
    for i in 0..20 {
        c.spawn((
            Caption(i),
            Text::new(""),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(GREEN),
            Node {
                position_type: PositionType::Absolute,
                padding: UiRect::axes(px(7), px(4)),
                border_radius: BorderRadius::all(px(4)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.04, 0.055, 0.82)),
            ZIndex(28),
        ));
    }
}

#[derive(Default)]
pub(crate) struct Ink(Vec<(Vec2, Vec2, Color)>);
impl Ink {
    fn line(&mut self, a: Vec2, b: Vec2, color: Color) {
        self.0.push((a, b, color));
    }
    fn ring(&mut self, p: Vec2, r: f32, fraction: f32, color: Color) {
        for i in 0..(36.0 * fraction.clamp(0.0, 1.0)).ceil() as usize {
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / 36.0;
            let b = a + std::f32::consts::TAU / 36.0;
            self.line(
                p + Vec2::new(a.cos(), a.sin()) * r,
                p + Vec2::new(b.cos(), b.sin()) * r,
                color,
            );
        }
    }
}

pub fn draw(
    f: Res<Flight>,
    v: Res<ViewState>,
    combat: Res<Combat>,
    cameras: Query<(&Camera, &GlobalTransform), With<FlightCamera>>,
    windows:Query<&Window,With<bevy::window::PrimaryWindow>>,
    mut strokes: Query<
        (&Stroke, &mut Node, &mut UiTransform, &mut BackgroundColor),
        Without<Caption>,
    >,
    mut captions: Query<(&Caption, &mut Node, &mut Text, &mut TextColor), Without<Stroke>>,
    mut ink: Local<Ink>,
    crew: Option<Res<crate::crew::Crew>>,
    panel_sizes: (
        Query<&ComputedNode, With<super::CombatPanel>>,
        Query<&ComputedNode, With<super::dogfight::DefensePanel>>,
    ),
) {
    ink.0.clear();
    let mut labels: [Option<(Vec2, String, Color)>; 20] = std::array::from_fn(|_|None);
    let mut threat_docked = false;
    let active = combat.active
        && v.ship_follow.is_none()
        && v.index != 3
        && combat.bvr.as_ref().is_none_or(|b|b.mountain)
        && !v.map
        && !f.crashed
        && f.vehicle == Vehicle::Gripen
        && v.missile_follow.is_none();
    if let Some((camera, transform)) = cameras.iter().next().filter(|_| active) {
        let size = windows.single().map(|w|Vec2::new(w.width(),w.height()))
            .unwrap_or(Vec2::new(1440.,900.));
        let min = if v.simple {
            Vec2::new(32.0, 70.0)
        } else {
            Vec2::new((size.x * 0.25).min(360.0), (size.y * 0.255).min(230.0))
        };
        let max = if v.simple {
            Vec2::new(size.x - 32.0, size.y - 100.0).max(min)
        } else {
            Vec2::new(
                size.x - (size.x * 0.245).min(350.0),
                size.y - (size.y * 0.283).min(255.0),
            )
        };
        let project = |point| {
            crate::environment::graphics::project_to_ui(camera,transform,world::local(point,&f),size)
        };
        let mountain=combat.bvr.as_ref().is_some_and(|b|b.mountain);
        for(index,target)in combat.targets.iter().enumerate().filter(|(i,t)|t.alive
            && if mountain{!combat.bvr.as_ref().unwrap().friendly(*i)}else{*i==combat.selected}).take(16){
            let selected=index==combat.selected;
            let label=if selected{0}else{4+index.min(15)};
            let screen =
                project(target.position).filter(|p| p.cmpge(min).all() && p.cmple(max).all());
            let color = if !selected{RED}else if combat.cue(&f).is_err() {
                AMBER
            } else {
                GREEN
            };
            if let Some(p) = screen {
                let progress = if selected{(combat.lock / combat.weapon.lock_time()).clamp(0.0, 1.0) as f32}else{0.};
                if selected{
                    ink.ring(p, 24.0, 1.0, DIM);
                    ink.ring(p, 28.0, progress, color);
                }else{
                    for(a,b)in [(Vec2::X,Vec2::Y),(Vec2::Y,-Vec2::X),(-Vec2::X,-Vec2::Y),(-Vec2::Y,Vec2::X)]{
                        ink.line(p+a*12.,p+b*12.,color);
                    }
                }
                labels[label] = Some((
                    p + Vec2::new(if p.x > max.x - 160.0 { -175.0 } else { 35.0 }, 12.0),
                    format!(
                        "T{:02}  {:.2} km\nVc {:+.0} m/s  {}",
                        index + 1,
                        target.position.distance(f.position) / 1000.0,
                        closing(&f, target),
                        if progress >= 1.0 { "LOCK" } else if selected{"SELECTED"}else{"HOSTILE"}
                    ),
                    color,
                ));
            } else {
                // Works for targets behind the eye as well as outside its view.
                let local = transform.rotation().inverse()
                    * (world::local(target.position, &f) - transform.translation());
                let direction = Vec2::new(local.x, -local.y).normalize_or(Vec2::Y);
                let centre = (min + max) * 0.5;
                let half = (max - min) * 0.5;
                let factor = (half.x / direction.x.abs().max(0.001))
                    .min(half.y / direction.y.abs().max(0.001));
                let p = centre + direction * (factor - 18.0);
                let side = Vec2::new(-direction.y, direction.x);
                let arrow_color=if selected{AMBER}else{RED};
                ink.line(p, p - direction * 14.0 + side * 7.0, arrow_color);
                ink.line(p, p - direction * 14.0 - side * 7.0, arrow_color);
                labels[label] = Some((
                    (p + Vec2::new(-55.0, 18.0))
                        .clamp(min, (max - Vec2::new(150.0, 45.0)).max(min)),
                    format!(
                        "{}T{:02}  {:.1} km\n{}",
                        if selected{"* "}else{""},
                        index + 1,
                        target.position.distance(f.position) / 1000.0,
                        if local.z > 0.0 {
                            "BEHIND"
                        } else {
                            "OFF SCREEN"
                        }
                    ),
                    arrow_color,
                ));
            }
        }
        // Separate closely spaced formation labels without moving their
        // geographic anchors or the direction arrows.
        let mut occupied:Vec<Vec2>=Vec::new();
        for index in std::iter::once(0).chain(4..labels.len()){
            if let Some((p,_,color))=&mut labels[index]{
                let original=*p;
                for row in 0..12{
                    let offset=if row%2==0{(row/2)as f32}else{-((row+1)/2)as f32}*46.;
                    let candidate=(original+Vec2::Y*offset).clamp(Vec2::new(24.,75.),(size-Vec2::new(220.,145.)).max(Vec2::new(24.,75.)));
                    let over_status=mountain&&candidate.x+205.>size.x-450.&&candidate.x<size.x-20.
                        &&candidate.y+43.>165.&&candidate.y<260.;
                    if !over_status&&occupied.iter().all(|q|(q.x-candidate.x).abs()>205.||(q.y-candidate.y).abs()>43.){
                        *p=candidate;break;
                    }
                }
                if p.distance(original)>8.{ink.line(original,*p,*color);}
                occupied.push(*p);
            }
        }
        if combat.fight.is_some() {
            let portrait =
                crate::crew::layout::portrait_bounds(crew.as_deref(), &f, &v, &combat, size);
            let weapon_height = panel_sizes
                .0
                .iter()
                .next()
                .map_or(132., |p| (p.size().y * p.inverse_scale_factor()).max(132.));
            let defense_height = panel_sizes
                .1
                .iter()
                .next()
                .map_or(164., |p| (p.size().y * p.inverse_scale_factor()).max(164.));
            let (centre, warning_position, docked) = crate::crew::layout::threat_positions(
                size,
                v.simple,
                portrait,
                crate::crew::layout::COMBAT_TOP + weapon_height + 12. + defense_height,
            );
            threat_docked = docked;
            let (count, range, _, kind) = combat.threats(&f);
            if !v.simple || count > 0 {
                ink.ring(centre, 34.0, 1.0, DIM);
                ink.line(centre - Vec2::Y * 42.0, centre - Vec2::Y * 31.0, GREEN);
            }
            for missile in &combat.fight.as_ref().unwrap().enemy_missiles {
                if !missile.alive || (missile.decoy.is_some() && missile.kind != Weapon::Heat) {
                    continue;
                }
                let body = f.orientation.conjugate() * (missile.position - f.position);
                let bearing = body.x.atan2(-body.z) as f32;
                let direction = Vec2::new(bearing.sin(), -bearing.cos());
                let approaching =
                    (missile.velocity - f.velocity).dot(f.position - missile.position) > 0.0;
                ink.line(
                    centre + direction * 25.0,
                    centre + direction * 38.0,
                    if approaching { RED } else { AMBER },
                );
            }
            if !v.simple || count > 0 {
                labels[2] = Some((
                    warning_position,
                    if let Some(kind) = kind {
                        format!(
                            "{} INBOUND  {:.1} km\n{}",
                            count,
                            range / 1000.0,
                            if kind == Weapon::Heat {
                                "IR / V FLARE + CHAFF"
                            } else {
                                "RADAR / BKSP CHAFF"
                            }
                        )
                    } else {
                        "THREAT BEARING\nNO MISSILE CLOSING".into()
                    },
                    if count > 0 { RED } else { DIM },
                ));
            }
            if v.simple && v.index != 3 {
                let fight = combat.fight.as_ref().unwrap();
                labels[3] = Some((
                    Vec2::new(size.x * 0.5 - 185.0, size.y - 52.0),
                    format!(
                        "{} {}  ·  U GUN {}  ·  V BOTH F{}/C{}  ·  BKSP CHAFF",
                        if combat.weapon == Weapon::Heat {
                            "IR"
                        } else {
                            "RAD"
                        },
                        combat.ammo[combat.weapon.index()],
                        fight.gun,
                        fight.flares,
                        fight.chaff
                    ),
                    GREEN,
                ));
            }
        }
    }
    if v.index == 3 {
        labels[0] = None;
        labels[1] = None;
    }
    for (tag, mut node, mut transform, mut color) in &mut strokes {
        if node.display != Display::None {
            node.display = Display::None;
        }
        if let Some((a, b, c)) = ink.0.get(tag.0 / 2) {
            let mid = (*a + *b) * 0.5;
            let delta = *b - *a;
            let length = delta.length().max(1.0);
            node.display = Display::Flex;
            node.left = px(mid.x - length * 0.5);
            let thickness = if tag.0 % 2 == 0 { 3.6 } else { 1.6 };
            node.top = px(mid.y - thickness * 0.5);
            node.width = px(length);
            node.height = px(thickness);
            transform.rotation = Rot2::radians(delta.y.atan2(delta.x));
            color.0 = if tag.0 % 2 == 0 {
                Color::srgba(0.015, 0.045, 0.04, 0.50)
            } else {
                *c
            };
        }
    }
    for (tag, mut node, mut text, mut color) in &mut captions {
        if node.display != Display::None {
            node.display = Display::None;
        }
        if tag.0 == 2 {
            let width = if threat_docked { px(170) } else { Val::Auto };
            if node.width != width {
                node.width = width;
            }
        }
        if let Some((p, t, c)) = &labels[tag.0] {
            node.display = Display::Flex;
            node.left = px(p.x);
            node.top = px(p.y);
            if &text.0 != t {
                text.0.clone_from(t);
            }
            color.0 = *c;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn close_gun_lead_subtracts_shooters_inherited_lateral_velocity(){
        let mut f=super::super::flight();
        f.velocity+=f.orientation*DVec3::X*250.;
        let position=f.position+f.forward()*1000.;
        let solution=gun_intercept(&f,position,f.velocity).unwrap();
        assert!(solution.direction.dot(f.orientation*DVec3::X).abs()<0.002,
            "co-moving aircraft must not require lateral lead");
        let crossing=gun_intercept(&f,position,f.velocity+f.orientation*DVec3::X*100.).unwrap();
        assert!(crossing.direction.dot(f.orientation*DVec3::X)>0.02);
    }
    #[test]
    fn gun_ranging_skips_friendlies_rear_destroyed_and_terrain_masked_targets(){
        let setup=super::super::mountain::Setup{force:3,entry:0,difficulty:super::super::dogfight::Difficulty::Balanced};
        let mut f=super::super::flight();f.position=f.position.normalize()*(planet_radius()+5000.);
        let mut c=Combat::default();super::super::mountain::start(&mut c,&f,1,setup);
        for t in &mut c.targets{t.position=f.position-f.forward()*1000.;}
        c.targets[0].position=f.position+f.forward()*900.;
        c.targets[1].position=f.position+f.forward()*1000.+f.orientation*DVec3::X*200.;
        c.targets[2].position=f.position+f.forward()*300.; // friendly wingman
        c.selected=2;
        assert_eq!(gun_target(&c,&f).map(|(i,_)|i),Some(0));
        c.selected=1;assert_eq!(gun_target(&c,&f).map(|(i,_)|i),Some(1));
        c.targets[1].alive=false;assert_eq!(gun_target(&c,&f).map(|(i,_)|i),Some(0));
        c.targets[0].position=f.position.normalize()*(planet_radius()-100.);
        assert!(gun_target(&c,&f).is_none());
    }
    #[test]
    fn director_on_current_target_corresponds_to_a_round_hitting_future_target() {
        let mut f = super::super::flight();
        let mut combat = Combat::default(); combat.start(&f, 1);
        let mut target = combat.targets[0].clone();
        target.position = f.position + f.forward() * 800.;
        target.velocity = f.velocity + f.orientation * DVec3::X * 90.;
        // Aim using the independently solved intercept, updating muzzle offset.
        for _ in 0..5 {
            let solution = gun_solution(&f, &target).unwrap();
            f.orientation = DQuat::from_rotation_arc(-DVec3::Z, solution.direction);
        }
        let cue = gun_pipper(&f, &target).unwrap();
        assert!(cue.aligned);
        assert!(cue.impact.distance(target.position) < 0.002);
        let mut bullet = f.position + f.orientation * crate::aircraft::gun_origin(f.aircraft);
        let mut velocity = f.velocity + f.forward() * CANNON_MUZZLE_SPEED;
        let dt = cue.time / 10000.;
        for _ in 0..10000 {
            velocity -= f.up() * 9.81 * dt;
            bullet += velocity * dt;
        }
        assert!(bullet.distance(target.position + target.velocity * cue.time) < 0.003);
    }
    #[test]
    fn director_requires_lead_and_rejects_invalid_or_rear_targets() {
        let f = super::super::flight();
        let mut combat = Combat::default(); combat.start(&f, 1);
        let mut target = combat.targets[0].clone();
        target.position = f.position + f.forward() * 1000.;
        target.velocity = f.velocity + f.orientation * DVec3::X * 120.;
        let cue = gun_pipper(&f, &target).unwrap();
        let right = f.orientation * DVec3::X;
        assert!((cue.impact - cue.bore).dot(right) < -20.);
        assert!((cue.impact - cue.bore).dot(f.up()) < 0.);
        assert!(!cue.aligned);
        target.position = f.position - f.forward() * 500.;
        assert!(gun_pipper(&f, &target).is_none());
        target.position = f.position + f.forward() * 2500.;
        assert!(gun_pipper(&f, &target).is_none());
        target.position = f.position + f.forward() * 500.; target.alive = false;
        assert!(gun_pipper(&f, &target).is_none());
    }
    #[test]
    fn lead_solution_matches_the_actual_gravity_and_inherited_velocity() {
        let f = super::super::flight();
        let mut combat = Combat::default();
        combat.start(&f, 1);
        let mut target = combat.targets[0].clone();
        target.velocity += f.orientation * DVec3::X * 90.0;
        let s = gun_solution(&f, &target).unwrap();
        let muzzle = f.position + f.orientation * crate::aircraft::gun_origin(f.aircraft);
        let bullet = muzzle + (f.velocity + s.direction * CANNON_MUZZLE_SPEED) * s.time
            - f.up() * 4.905 * s.time * s.time;
        let predicted = target.position + target.velocity * s.time;
        assert!(bullet.distance(predicted) < 0.001);
        let line_of_sight = (target.position - muzzle).normalize();
        assert!((s.direction - line_of_sight).dot(f.orientation * DVec3::X) > 0.0);
    }
    #[test]
    fn unreachable_and_destroyed_targets_have_no_gun_cue() {
        let f = super::super::flight();
        let mut combat = Combat::default();
        combat.start(&f, 1);
        let mut target = combat.targets[0].clone();
        target.velocity = f.velocity + f.forward() * (CANNON_MUZZLE_SPEED * 1.2);
        assert!(gun_solution(&f, &target).is_none());
        target.alive = false;
        assert!(gun_solution(&f, &target).is_none());
    }
    #[test]
    fn hornet_close_gun_cue_uses_its_actual_muzzle() {
        let mut f=super::super::flight();crate::aircraft::select(&mut f,crate::aircraft::Aircraft::Hornet);
        f.rates=DVec3::ZERO;
        let points=funnel(&f,2.5);let midpoint=(points[0][0]+points[0][1])*0.5;
        let expected=f.position+f.orientation*crate::aircraft::gun_origin(f.aircraft)
            +f.forward()*30.-f.up()*4.905*(30./CANNON_MUZZLE_SPEED).powi(2);
        assert!(midpoint.distance(expected)<1e-6,"close funnel does not match Hornet muzzle: error {} m",midpoint.distance(expected));
    }
    #[test]
    fn funnel_uses_wingspan_gravity_and_turn_rate() {
        let mut f = super::super::flight();
        f.rates = DVec3::ZERO;
        let straight = funnel(&f, 8.4);
        for rails in &straight {
            assert!((rails[0].distance(rails[1]) - 8.4).abs() < 1e-6);
        }
        let centre = (straight[28][0] + straight[28][1]) * 0.5;
        let expected = f.position + f.orientation * crate::aircraft::gun_origin(f.aircraft)+f.forward()*900.
            - f.up() * 4.905 * (900.0_f64 / CANNON_MUZZLE_SPEED).powi(2);
        assert!(centre.distance(expected) < 1e-6);
        f.rates.x = 0.2;
        let turning = funnel(&f, 8.4);
        assert!(turning[28][0].distance(straight[28][0]) > 5.0);
        assert!(turning.iter().flatten().all(|p| p.is_finite()));
    }
}
