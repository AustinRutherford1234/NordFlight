//! Bounded simulation-time combat particles and a component damage schematic.
use super::{
    Combat,
    damage::SurfaceState,
};
use crate::{ViewState, physics::*, world};
use bevy::{
    asset::RenderAssetUsages,
    math::DVec3,
    prelude::*,
    render::render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat},
};

#[derive(Clone, Copy)]
pub enum Kind {
    Fragment,
    Smoke,
    Flame,
    Flash,
    Spark,
    Spray,
    Foam,
}
pub struct Particle {
    pub position: DVec3,
    pub velocity: DVec3,
    pub age: f64,
    pub life: f64,
    pub size: Vec3,
    pub kind: Kind,
    /// Presentation identity only: lets the renderer keep a particle on the
    /// same entity while the simulation vector shifts underneath it.
    pub id: u64,
}
#[derive(Default)]
pub struct State {
    pub particles: Vec<Particle>,
    next_id: u64,
    water_seen: bool,
    seen: [[bool; crate::aircraft::SURFACES]; 12],
    seen_wings: [[bool; 2]; 12],
    dead: [bool; 12],
    clock: f64,
    emission: f64,
}
impl State {
    fn emit(&mut self, p: DVec3, v: DVec3, kind: Kind, size: Vec3, life: f64) {
        if self.particles.len() >= 256 {
            self.particles.remove(0);
        }
        self.next_id += 1;
        self.particles.push(Particle {
            id: self.next_id,
            position: p,
            velocity: v,
            kind,
            size,
            age: 0.0,
            life,
        });
    }
    fn aircraft(
        &mut self,
        id: usize,
        p: DVec3,
        v: DVec3,
        q: bevy::math::DQuat,
        d: &AircraftDamage,
        dead: bool,
        emit: bool,
    ) {
        for side in 0..2 {
            if d.wing_broken[side] && !self.seen_wings[id][side] {
                let sign = if side == 0 { -1. } else { 1. };
                self.emit(p + q * DVec3::new(sign * 4.2, 0., 0.8),
                    v + q * DVec3::new(sign * 3., 0., 1.), Kind::Fragment,
                    Vec3::new(3.0, 0.12, 2.0), 20.);
            }
            self.seen_wings[id][side] = d.wing_broken[side];
        }
        for i in 0..crate::aircraft::count(d.aircraft) {
            let detached = d.surface_state[i] == SurfaceState::Detached;
            if detached && !self.seen[id][i] {
                self.emit(
                    p + q * crate::aircraft::surface_points(d.aircraft)[i],
                    v + q * DVec3::new(if i < 4 { -4.0 } else { 4.0 }, 2.0, 1.0),
                    Kind::Fragment,
                    Vec3::new(0.8, 0.07, 0.9),
                    16.0,
                );
            }
            self.seen[id][i] = detached;
        }
        if dead && !self.dead[id] {
            self.emit(p, v, Kind::Flash, Vec3::splat(2.0), 0.6);
            for i in 0..18 {
                let a = i as f64 * 2.39996;
                let scatter = q * DVec3::new(a.cos(), (i % 4) as f64 * 0.4, a.sin());
                self.emit(
                    p + scatter * 2.0,
                    v + scatter * (8.0 + (i % 3) as f64 * 5.0),
                    Kind::Fragment,
                    Vec3::new(0.6 + (i % 3) as f32 * 0.4, 0.15, 0.9 + (i % 4) as f32 * 0.3),
                    18.0,
                );
            }
        }
        self.dead[id] = dead;
        if emit {
            let strength = if dead {
                1.0
            } else {
                (d.fire + (1.0 - d.engine) * 0.3 + d.fuel_leak * 0.08).min(1.0)
            };
            if strength > 0.04 {
                let origin = p + q * if d.aircraft!=crate::aircraft::Aircraft::Gripen {
                    if d.fuel_leak>0.3{DVec3::new(-d.aircraft.spec().span*0.2,0.1,0.5)}else{d.aircraft.engine_positions()[usize::from(d.engines[1]<d.engines[0])]}
                } else if d.fuel_leak > 0.3 {
                    DVec3::new(-1.6, 0.1, 1.4)
                } else {
                    DVec3::new(0.0, 0.3, 4.5)
                };
                self.emit(
                    origin,
                    v * 0.65 + p.normalize() * 2.0,
                    Kind::Smoke,
                    Vec3::splat(0.6 + strength as f32),
                    3.8,
                );
                if d.fire > 0.08 || dead {
                    self.emit(
                        origin,
                        // Short-lived flame tongues inherit the aircraft motion;
                        // slower smoke separates into the wake farther downstream.
                        v + q * DVec3::Z * 4.0,
                        Kind::Flame,
                        Vec3::new(0.25, 0.7, 0.25),
                        0.22,
                    );
                    let phase = self.clock * 31.0 + id as f64 * 2.39996;
                    self.emit(
                        origin,
                        v * 0.94 + q * DVec3::new(phase.sin() * 7., 3. + phase.cos() * 3., 12.),
                        Kind::Spark,
                        Vec3::new(0.025, 0.10, 0.025),
                        0.65,
                    );
                }
            }
        }
    }
    pub fn advance(&mut self, c: &Combat, f: &Flight, dt: f64) {
        self.clock += dt;
        if f.water.active && !self.water_seen {
            let up=f.water.origin.normalize();
            let right=up.cross(DVec3::Z).normalize_or_zero();let forward=right.cross(up);
            let energy=(f.water.sink*1.5+f.water.speed*0.12).clamp(3.,28.);
            for i in 0..90 {
                let angle=i as f64*2.39996;
                let radial=right*angle.cos()+forward*angle.sin();
                let vertical=energy*(0.45+(i%9) as f64/12.);
                self.emit(f.water.origin+radial*(i%7) as f64*0.35+up*0.25,
                    radial*(2.+(i%8) as f64*0.8)+up*vertical,
                    Kind::Spray,Vec3::new(0.65,1.8,0.65)*(0.7+(i%4) as f32*0.18),5.);
            }
            for i in 0..28 {
                let a=i as f64*2.39996;let radial=right*a.cos()+forward*a.sin();
                self.emit(f.water.origin+radial*2.+up*0.08,radial*3.,Kind::Foam,Vec3::new(2.,2.,0.12),7.);
            }
        }
        self.water_seen=f.water.active;
        self.emission += dt;
        let emit = self.emission >= 0.10;
        if emit {
            self.emission %= 0.10;
        }
        for part in &mut self.particles {
            part.age += dt;
            let up = part.position.normalize();
            match part.kind {
                Kind::Fragment | Kind::Spark | Kind::Spray => {
                    part.velocity -= up * 9.81 * dt;
                    part.velocity *= (-dt * 0.04).exp();
                }
                Kind::Foam => {
                    part.velocity *= (-dt*0.7).exp();
                    part.position=up*(RADIUS+crate::ocean::sample(part.position,f.elapsed).height+0.08);
                }
                Kind::Smoke => {
                    part.velocity *= (-dt * 0.9).exp();
                    part.velocity += up * 2.0 * dt;
                }
                _ => {}
            }
            let before=part.position;
            part.position += part.velocity * dt;
            if matches!(part.kind,Kind::Fragment|Kind::Spark) {
                if let Some(hit)=crate::carrier::collider::sweep(f,before,part.position,dt,0.2) {
                    part.position=hit.point+hit.normal*0.01;
                    let ship=crate::carrier::velocity(f.elapsed);let rel=part.velocity-ship;
                    part.velocity=ship+(rel-hit.normal*rel.dot(hit.normal).min(0.)*1.2)*(-dt*2.).exp();
                }
            }
            if matches!(part.kind, Kind::Spray) && part.age>0.1 && part.position.length()<RADIUS+3. && part.position.length()<RADIUS+crate::ocean::sample(part.position,f.elapsed).height {part.age=part.life;}
            if matches!(part.kind, Kind::Fragment) {
                let ground = if in_lake(part.position.x,part.position.z)||crate::carrier::sea(part.position.x,part.position.z) {crate::ocean::sample(part.position,f.elapsed).height+0.2} else {terrain_height(part.position.x, part.position.z).max(0.0) + 0.2};
                if part.position.length() - RADIUS < ground {
                    part.position = up * (RADIUS + ground);
                    part.velocity -= up * part.velocity.dot(up).min(0.0) * 1.2;
                    part.velocity *= (-dt * 2.0).exp();
                }
            }
        }
        self.particles.retain(|p| p.age < p.life);
        // The dedicated Hornet wreck scene owns player breakup/fire; keep AI effects
        // and the existing water plume, without a second overlapping explosion.
        if !(f.aircraft == crate::aircraft::Aircraft::Hornet && f.vehicle == Vehicle::Gripen && f.crashed) {
            self.aircraft(
                0,
                f.position,
                f.velocity,
                f.orientation,
                &f.damage,
                f.crashed && !f.water.active,
                emit && !f.water.active,
            );
        }
        for (i, t) in c.targets.iter().take(11).enumerate() {
            self.aircraft(
                i + 1,
                t.position,
                t.velocity,
                t.orientation,
                &t.damage,
                !t.alive,
                emit && (t.alive || t.wreck_age < 10.0),
            );
        }
    }
}
pub fn advance(c: &mut Combat, f: &Flight, dt: f64) {
    if f.paused || dt <= 0.0 || (!c.active && !f.water.active && !f.crashed && !f.damage.wing_broken.iter().any(|&broken| broken) && c.fx.particles.is_empty()) {
        return;
    }
    let mut state = std::mem::take(&mut c.fx);
    state.advance(c, f, dt);
    c.fx = state;
}
pub fn controls(
    c: Res<Combat>,
    mut surfaces: Query<(&super::TargetControl, &mut Transform, &mut Visibility)>,
) {
    for (part, mut transform, mut visibility) in &mut surfaces {
        let target = c.targets.get(part.target).filter(|t| t.alive && c.active);
        *visibility = Visibility::Hidden;
        if let Some(t) = target {
            if t.damage.surface_state[part.index] != SurfaceState::Detached {
                *visibility = Visibility::Inherited;
                transform.rotation = Quat::from_axis_angle(
                    part.axis,
                    crate::control_surfaces::effective_angle(
                        &t.damage,
                        part.index,
                        crate::control_surfaces::angles(
                            t.dynamics.surface,
                            t.dynamics.flaps,
                            t.dynamics.airbrake,
                        )[part.index],
                    ) as f32,
                );
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn water_impact_emits_spray_and_foam_without_a_fireball() {
        let mut f=Flight::approach();f.water.active=true;f.water.origin=f.up()*RADIUS;f.water.speed=120.;f.water.sink=12.;f.crashed=true;f.damage.fire=1.;
        let mut state=State::default();let c=Combat::default();
        state.advance(&c,&f,0.01);
        assert!(state.particles.iter().any(|p|matches!(p.kind,Kind::Spray)));
        assert!(state.particles.iter().any(|p|matches!(p.kind,Kind::Foam)));
        assert!(!state.particles.iter().any(|p|matches!(p.kind,Kind::Flash|Kind::Flame|Kind::Smoke)));
        for _ in 0..900 {state.advance(&c,&f,0.01);}
        assert!(state.particles.is_empty());
    }
    #[test]
    fn broken_wing_emits_once_in_noncombat_flight() {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::F22);
        f.paused = false;
        let mut c = Combat::default();
        assert!(!c.active);
        f.damage.wing_broken[0] = true;
        advance(&mut c, &f, 0.01);
        let count = c.fx.particles.len();
        assert_eq!(count, 1);
        advance(&mut c, &f, 0.01);
        assert_eq!(c.fx.particles.len(), count);
    }
    #[test]
    fn fleet_damage_silhouettes_fit_the_texture_and_include_engine_and_vector_failures(){
        let mut images=std::collections::HashSet::new();
        for a in [crate::aircraft::Aircraft::Su27,crate::aircraft::Aircraft::F35B,crate::aircraft::Aircraft::F22,crate::aircraft::Aircraft::Hornet]{
            let mut d=AircraftDamage{aircraft:a,..default()};let mut pixels=vec![0;220*100*4];draw_diagram(&mut pixels,&d,1.,0.);let rows:Vec<_>=pixels.chunks_exact(220*4).enumerate().filter(|(_,r)|r.chunks_exact(4).any(|p|p[3]>0)).map(|(i,_)|i).collect();assert!(rows[0]>2);assert!(*rows.last().unwrap()<98);assert!(images.insert(pixels));
            d.engines[0]=0.;d.nozzles[0]=0.;d.lift_fan=0.;let label=diagram_label(&d,0.8,false);assert!(label.contains("ENGINE 1"));if a==crate::aircraft::Aircraft::F22{assert!(label.contains("VECTOR 1"));}
        }
    }
    #[test]
    fn flames_stay_near_the_damaged_region_at_high_airspeed() {
        let mut f = Flight::runway();
        f.velocity = f.forward() * 500.;
        f.damage.fire = 0.8;
        f.damage.fuel_leak = 1.;
        let c = Combat::default();
        let mut state = State::default();
        state.advance(&c, &f, 0.1);
        for _ in 0..10 {
            f.position += f.velocity * 0.01;
            state.advance(&c, &f, 0.01);
        }
        let source = f.position + f.orientation * DVec3::new(-1.6, 0.1, 1.4);
        let flames: Vec<_> = state
            .particles
            .iter()
            .filter(|p| matches!(p.kind, Kind::Flame))
            .collect();
        assert!(!flames.is_empty());
        assert!(flames.iter().all(|p| p.position.distance(source) < 2.));
    }
    #[test]
    fn bounded_polygons_match_full_texture_rasterization() {
        // Preserve the old pixel rule, including clipped and subpixel edges.
        for points in [
            vec![(103., 31.), (25., 81.), (101., 77.)],
            vec![(-8.3, -4.2), (230.1, 5.6), (100.4, 110.2)],
            vec![
                (124., 85.),
                (126., 69.3),
                (132., 77.),
                (134., 60.7),
                (139., 84.),
            ],
            vec![(230., 110.), (240., 110.), (235., 120.)],
            vec![],
        ] {
            let mut reference = vec![0; 220 * 100 * 4];
            let mut bounded = reference.clone();
            let color = [255, 110, 20, 255];
            polygon(&mut bounded, &points, color);
            for y in 0..100 {
                for x in 0..220 {
                    let mut inside = false;
                    for i in 0..points.len() {
                        let a = points[i];
                        let b = points[(i + 1) % points.len()];
                        if (a.1 > y as f32) != (b.1 > y as f32)
                            && (x as f32) < (b.0 - a.0) * (y as f32 - a.1) / (b.1 - a.1) + a.0
                        {
                            inside = !inside;
                        }
                    }
                    if inside {
                        let i = (y * 220 + x) * 4;
                        reference[i..i + 4].copy_from_slice(&color);
                    }
                }
            }
            assert_eq!(bounded, reference);
        }
    }
    #[test]
    fn particles_preserve_momentum_are_bounded_and_restart_clears_them() {
        let mut f = super::super::flight();
        let mut c = Combat::default();
        c.start(&f, 1);
        f.damage.surface_state[2] = SurfaceState::Detached;
        f.damage.fire = 0.7;
        advance(&mut c, &f, 0.01);
        let fragment =
            c.fx.particles
                .iter()
                .find(|p| matches!(p.kind, Kind::Fragment))
                .unwrap();
        assert!(fragment.velocity.distance(f.velocity) < 10.0);
        let age = fragment.age;
        f.paused = true;
        advance(&mut c, &f, 1.0);
        assert_eq!(c.fx.particles[0].age, age);
        f.paused = false;
        for _ in 0..5000 {
            advance(&mut c, &f, 0.01);
        }
        assert!(c.fx.particles.len() <= 256);
        c.start(&f, 2);
        assert!(c.fx.particles.is_empty());
    }
    #[test]
    fn hit_report_holds_three_seconds_fades_and_preserves_snapshot_until_reset() {
        let mut f = super::super::flight();
        let mut c = Combat::default();
        c.start(&f, 1);
        c.clock = 10.;
        c.targets[0].damage.surface_state[2] = SurfaceState::Jammed(0.25);
        c.recent_hit = Some(HitReport::new(0, &c.targets[0], c.clock));
        let r = c.recent_hit.as_ref().unwrap();
        assert_eq!(r.alpha(10.), 1.);
        assert_eq!(r.alpha(13.), 1.);
        assert!((r.alpha(13.325) - 0.5).abs() < 1e-5);
        assert_eq!(r.alpha(13.65), 0.);
        f.paused = true;
        c.tick(&mut f, 5.);
        assert_eq!(c.clock, 10.);
        c.selected = 1;
        c.targets[0].damage.surface_state[2] = SurfaceState::Detached;
        assert_eq!(
            c.recent_hit.as_ref().unwrap().damage.surface_state[2],
            SurfaceState::Jammed(0.25)
        );
        c.recent_hit = Some(HitReport::new(1, &c.targets[1], 12.));
        assert_eq!(c.recent_hit.as_ref().unwrap().index, 1);
        assert_eq!(c.recent_hit.as_ref().unwrap().alpha(15.), 1.);
        c.start(&f, 2);
        assert!(c.recent_hit.is_none());
    }
    #[test]
    fn hit_panel_is_visible_in_simple_cockpit_and_hidden_on_map_and_after_fade() {
        let f = super::super::flight();
        let mut combat = Combat::default();
        combat.start(&f, 1);
        prepare_hit_capture(&mut combat, "hit-report");
        let mut images = Assets::<Image>::default();
        let handle = images.add(Image::new_fill(
            Extent3d {
                width: 220,
                height: 100,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0; 4],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ));
        let mut app = App::new();
        app.insert_resource(f)
            .insert_resource(combat)
            .insert_resource(ViewState {
                index: 3,
                simple: true,
                ..default()
            })
            .insert_resource(images)
            .insert_resource(HitTexture(handle))
            .init_resource::<crate::display_upload::DisplayUploads>()
            .add_systems(Update, render_hit);
        let panel = app
            .world_mut()
            .spawn((HitPanel, Node::default(), BackgroundColor::default()))
            .id();
        let text = app
            .world_mut()
            .spawn((HitText, Text::new(""), TextColor::default()))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Node>(panel).unwrap().display,
            Display::Flex
        );
        assert!(
            app.world()
                .get::<Text>(text)
                .unwrap()
                .0
                .contains("L OUTER ELEVON LOST")
        );
        assert_eq!(app.world().get::<Node>(panel).unwrap().top, px(60));
        app.world_mut().resource_mut::<ViewState>().simple = false;
        app.update();
        let full = app.world().get::<Node>(panel).unwrap();
        assert_eq!(full.left, px(264));
        assert_eq!(full.right, Val::Auto);
        app.world_mut().resource_mut::<ViewState>().map = true;
        app.update();
        assert_eq!(
            app.world().get::<Node>(panel).unwrap().display,
            Display::None
        );
        app.world_mut().resource_mut::<ViewState>().map = false;
        app.world_mut().resource_mut::<Combat>().clock += 4.;
        app.update();
        assert_eq!(
            app.world().get::<Node>(panel).unwrap().display,
            Display::None
        );
    }
}
#[derive(Component)]
pub struct Visual(usize);
#[derive(Component)]
pub struct Diagram;
#[derive(Component)]
pub struct DiagramText;
#[derive(Component)]
pub struct DiagramImage;
#[derive(Resource)]
pub struct DiagramTexture(pub Handle<Image>);
/// A hit-time snapshot survives target selection changes and aircraft breakup.
pub struct HitReport {
    pub index: usize,
    pub damage: AircraftDamage,
    pub integrity: f64,
    pub destroyed: bool,
    pub at: f64,
}
impl HitReport {
    pub fn new(index: usize, target: &super::Target, at: f64) -> Self {
        Self {
            index,
            damage: target.damage.clone(),
            integrity: target.health,
            destroyed: !target.alive,
            at,
        }
    }
    pub fn alpha(&self, now: f64) -> f32 {
        ((3.65 - (now - self.at).max(0.0)) / 0.65).clamp(0.0, 1.0) as f32
    }
}
#[derive(Component)]
pub struct HitPanel;
pub fn prepare_hit_capture(combat: &mut Combat, mode: &str) {
    let target = &mut combat.targets[0];
    target.damage.engine = 0.;
    target.damage.surface_state[2] = SurfaceState::Detached;
    target.damage.surfaces[2] = 0.;
    target.damage.surface_state[4] = SurfaceState::Jammed(0.1);
    target.damage.fuel_leak = 1.2;
    target.damage.fire = 0.6;
    target.health = 0.55;
    if mode.contains("destroyed") {
        target.health = 0.;
        target.alive = false;
    }
    let age = if mode.contains("fade") {
        3.325
    } else if mode.contains("expired") {
        4.
    } else {
        0.
    };
    combat.recent_hit = Some(HitReport::new(0, target, combat.clock - age));
}
#[derive(Component)]
pub struct HitText;
#[derive(Component)]
pub struct HitImage;
#[derive(Resource)]
pub struct HitTexture(Handle<Image>);
#[derive(Resource)]
pub struct Geometry {
    sphere: Handle<Mesh>,
    fragment: Handle<Mesh>,
}

pub fn setup(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut vertices = Vec::new();
    let mut colors = Vec::new();
    for i in 0..24 {
        let a = i as f32 * std::f32::consts::TAU / 24.0;
        let b = (i + 1) as f32 * std::f32::consts::TAU / 24.0;
        vertices.extend([
            [0.0, 0.0, 0.0],
            [a.cos(), a.sin(), 0.0],
            [b.cos(), b.sin(), 0.0],
        ]);
        colors.extend([
            [1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0, 0.0],
        ]);
    }
    let sphere = meshes.add(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 72])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; 72]),
    );
    // Folded, jagged sheet fragments with a charred outer face and exposed metal.
    let mut shard = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    shard.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-0.5, 0., -0.45],
            [0.28, -0.08, -0.5],
            [0.5, 0.03, -0.12],
            [0.34, 0.18, 0.5],
            [-0.35, 0.12, 0.38],
            [-0.10, -0.12, 0.0],
        ],
    );
    shard.insert_attribute(
        Mesh::ATTRIBUTE_COLOR,
        vec![
            [0.18, 0.15, 0.12, 1.],
            [0.65, 0.62, 0.56, 1.],
            [0.32, 0.34, 0.35, 1.],
            [0.68, 0.63, 0.51, 1.],
            [0.12, 0.13, 0.14, 1.],
            [0.30, 0.28, 0.23, 1.],
        ],
    );
    shard.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; 6]);
    shard.insert_indices(bevy::mesh::Indices::U32(vec![
        0, 1, 5, 1, 2, 5, 2, 3, 5, 3, 4, 5, 4, 0, 5,
    ]));
    shard.compute_smooth_normals();
    let cube = meshes.add(shard);
    c.insert_resource(Geometry {
        sphere: sphere.clone(),
        fragment: cube.clone(),
    });
    for i in 0..256 {
        c.spawn((
            Visual(i),
            Mesh3d(sphere.clone()),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::NONE,
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                cull_mode: None,
                ..default()
            })),
            Transform::default(),
            Visibility::Hidden,
            bevy::light::NotShadowCaster,
            bevy::camera::visibility::NoFrustumCulling,
        ));
    }
    let texture = images.add(Image::new_fill(
        Extent3d {
            width: 220,
            height: 100,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    c.insert_resource(DiagramTexture(texture.clone()));
    let hit_texture = images.add(Image::new_fill(
        Extent3d {
            width: 220,
            height: 100,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    c.insert_resource(HitTexture(hit_texture.clone()));
    c.spawn((
        HitPanel,
        Node {
            position_type: PositionType::Absolute,
            right: px(28),
            top: px(220),
            width: px(250),
            padding: UiRect::all(px(10)),
            flex_direction: FlexDirection::Column,
            row_gap: px(5),
            border_radius: BorderRadius::all(px(8)),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.015, 0.035, 0.05, 0.94)),
        ZIndex(33),
    ))
    .with_children(|p| {
        p.spawn((
            HitText,
            Text::new(""),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(1., 0.8, 0.55)),
        ));
        p.spawn((
            HitImage,
            ImageNode::new(hit_texture),
            Node {
                width: px(230),
                height: px(105),
                ..default()
            },
        ));
    });
    c.spawn((
        Diagram,
        Node {
            position_type: PositionType::Absolute,
            left: px(28),
            top: px(220),
            width: px(220),
            min_height: px(145),
            padding: UiRect::all(px(5)),
            flex_direction: FlexDirection::Column,
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.015, 0.035, 0.05, 0.90)),
        ZIndex(32),
    ))
    .with_children(|p| {
        p.spawn((
            Text::new("AIRFRAME / SYSTEMS"),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.92, 0.91)),
        ));
        p.spawn((
            DiagramImage,
            ImageNode::new(texture),
            Node {
                width: px(210),
                height: px(95),
                ..default()
            },
        ));
        p.spawn((
            DiagramText,
            Text::new(""),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.75, 0.5)),
        ));
    });
}
pub fn render_hit(
    f: Res<Flight>,
    v: Res<ViewState>,
    combat: Res<Combat>,
    texture: Res<HitTexture>,
    mut images: ResMut<Assets<Image>>,
    uploads: Res<crate::display_upload::DisplayUploads>,
    mut panels: Query<(&mut Node, &mut BackgroundColor), With<HitPanel>>,
    mut labels: Query<(&mut Text, &mut TextColor), With<HitText>>,
    mut pictures: Query<&mut ImageNode, With<HitImage>>,
    mut previous: Local<Option<(u64, usize, u64)>>,
) {
    let report = combat.recent_hit.as_ref();
    let alpha = report.map_or(0., |r| r.alpha(combat.clock));
    let visible = alpha > 0. && !v.map && f.vehicle == Vehicle::Gripen;
    for (mut node, mut background) in &mut panels {
        node.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
        // Keep clear of the full-view BVR status and flight tapes on the right.
        node.left = if v.simple { Val::Auto } else { px(264) };
        node.right = if v.simple { px(28) } else { Val::Auto };
        // Reserve the simplified-view status badge at the upper-right edge.
        node.top = px(if v.simple { 60 } else { 220 });
        background.0 = Color::srgba(0.015, 0.035, 0.05, 0.94 * alpha);
    }
    if !visible {
        *previous = None;
        return;
    }
    let r = report.unwrap();
    let key = (
        r.at.to_bits(),
        r.index,
        if r.damage.fire > 0.05 {
            combat.clock.to_bits()
        } else {
            0
        },
    );
    for mut picture in &mut pictures {
        picture.color = Color::WHITE.with_alpha(alpha);
    }
    for (mut text, mut color) in &mut labels {
        color.0 = Color::srgba(1., 0.8, 0.55, alpha);
        if *previous != Some(key) {
            text.0 = format!(
                "ENEMY {:02} / {}\n{}",
                r.index + 1,
                if r.destroyed {
                    "DESTROYED"
                } else {
                    "HIT CONFIRMED"
                },
                diagram_label(&r.damage, r.integrity, r.destroyed)
            );
        }
    }
    if *previous != Some(key) {
        if let Some(image) = images.get_mut_untracked(&texture.0) {
            if let Some(data) = image.data.as_mut() {
                draw_diagram(data, &r.damage, r.integrity, combat.clock);
            }
            uploads.publish(&texture.0, image);
        }
        *previous = Some(key);
    }
}
fn color(health: f64, state: SurfaceState) -> [u8; 4] {
    match state {
        SurfaceState::Jammed(_) => [237, 75, 222, 255],
        SurfaceState::Detached => [255, 65, 60, 255],
        _ => {
            if health < 0.15 {
                [255, 65, 60, 255]
            } else if health < 0.98 {
                [255, 185, 60, 255]
            } else {
                [64, 147, 155, 255]
            }
        }
    }
}
fn polygon(data: &mut [u8], points: &[(f32, f32)], c: [u8; 4]) {
    let minx = points
        .iter()
        .map(|p| p.0.floor() as i32)
        .min()
        .unwrap_or(0)
        .clamp(0, 219);
    let maxx = points
        .iter()
        .map(|p| p.0.ceil() as i32)
        .max()
        .unwrap_or(0)
        .clamp(0, 219);
    let miny = points
        .iter()
        .map(|p| p.1.floor() as i32)
        .min()
        .unwrap_or(0)
        .clamp(0, 99);
    let maxy = points
        .iter()
        .map(|p| p.1.ceil() as i32)
        .max()
        .unwrap_or(0)
        .clamp(0, 99);
    for y in miny..=maxy {
        for x in minx..=maxx {
            let mut inside = false;
            for i in 0..points.len() {
                let a = points[i];
                let b = points[(i + 1) % points.len()];
                if (a.1 > y as f32) != (b.1 > y as f32)
                    && (x as f32) < (b.0 - a.0) * (y as f32 - a.1) / (b.1 - a.1) + a.0
                {
                    inside = !inside;
                }
            }
            if inside {
                let idx = ((y * 220 + x) * 4) as usize;
                data[idx..idx + 4].copy_from_slice(&c);
            }
        }
    }
}
pub fn render(
    f: Res<Flight>,
    v: Res<ViewState>,
    combat: Res<Combat>,
    texture: Res<DiagramTexture>,
    mut images: ResMut<Assets<Image>>,
    uploads: Res<crate::display_upload::DisplayUploads>,
    mut diagrams: Query<&mut Node, With<Diagram>>,
    mut labels: Query<&mut Text, With<DiagramText>>,
    mut visuals: Query<(
        &Visual,
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Transform,
        &mut Visibility,
    )>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    geometry: Res<Geometry>,
    cameras: Query<&GlobalTransform, With<crate::FlightCamera>>,
    mut previous: Local<Option<u64>>,
    mut slots: Local<Vec<u64>>,
) {
    for mut node in &mut diagrams {
        node.display = if v.map || f.vehicle != Vehicle::Gripen {
            Display::None
        } else {
            Display::Flex
        };
        node.top = px(if v.simple { 20 } else { 220 });
    }
    let d = &f.damage;
    let mut signature = 0xcbf29ce484222325u64;
    for value in d.wings.iter().chain(d.surfaces.iter()).copied().chain([
        f.integrity,
        d.engine,
        d.hydraulics,
        d.radar,
        d.avionics,
        d.fuel_leak,
        if d.fire > 0.05 { f.elapsed + 1.0 } else { 0.0 },
        f.crashed as u8 as f64,
        d.aircraft as u8 as f64,d.engines[0],d.engines[1],d.nozzles[0],d.nozzles[1],d.lift_fan,
    ]) {
        signature = (signature ^ value.to_bits()).wrapping_mul(0x100000001b3);
    }
    for state in d.surface_state {
        signature = (signature
            ^ match state {
                SurfaceState::Operational => 0,
                SurfaceState::Degraded => 1,
                SurfaceState::Jammed(_) => 2,
                SurfaceState::Detached => 3,
            })
        .wrapping_mul(0x100000001b3);
    }
    if !v.map && f.vehicle == Vehicle::Gripen && *previous != Some(signature) {
        *previous = Some(signature);
        if let Some(image) = images.get_mut_untracked(&texture.0) {
            if let Some(data) = image.data.as_mut() {
                draw_diagram(data, d, f.integrity, f.elapsed);
            }
            uploads.publish(&texture.0, image);
        }
        let text = diagram_label(d, f.integrity, f.crashed);
        for mut label in &mut labels {
            if label.0 != text {
                label.0.clone_from(&text);
            }
        }
    }
    // Stable particle -> entity slots. Index binding shifted every live
    // particle onto a neighbouring entity whenever one expired, which swapped
    // meshes and rewrote every material.
    slots.resize(256, 0);
    let live: std::collections::HashMap<u64, &Particle> =
        combat.fx.particles.iter().map(|p| (p.id, p)).collect();
    for slot in slots.iter_mut() {
        if !live.contains_key(slot) {
            *slot = 0;
        }
    }
    for p in &combat.fx.particles {
        if !slots.contains(&p.id)
            && let Some(free) = slots.iter_mut().find(|s| **s == 0)
        {
            *free = p.id;
        }
    }
    for (tag, mut mesh, material, mut t, mut visibility) in &mut visuals {
        let particle = slots.get(tag.0).and_then(|id| live.get(id)).copied();
        visibility.set_if_neq(if particle.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let Some(p) = particle else {
            continue;
        };
        let desired_mesh = if matches!(p.kind, Kind::Fragment) {
            geometry.fragment.clone()
        } else {
            geometry.sphere.clone()
        };
        if mesh.0 != desired_mesh {
            mesh.0 = desired_mesh;
        }
        t.translation = world::local(p.position, &f);
        let fade = (1.0 - p.age / p.life).max(0.0) as f32;
        // Material colour on a 1/128 fade grid: alpha error is below 0.004
        // while unchanged particles no longer re-prepare their material.
        let tint = (fade * 128.0).round() / 128.0;
        t.scale = p.size
            * match p.kind {
                Kind::Smoke => 1.0 + p.age as f32 * 1.5,
                Kind::Spray => 1.0+p.age as f32*0.7,
                Kind::Foam => 1.0+p.age as f32*1.2,
                Kind::Flash => 1.0 + p.age as f32 * 8.0,
                Kind::Flame => 0.8 + 0.25 * (p.age as f32 * 60. + tag.0 as f32).sin(),
                _ => 1.0,
            };
        t.rotation =
            Quat::from_rotation_x(p.age as f32 * 2.0) * Quat::from_rotation_z(tag.0 as f32);
        if !matches!(p.kind, Kind::Fragment | Kind::Foam) {
            if let Some(camera) = cameras.iter().next() {
                t.rotation = camera.rotation();
            }
        }
        let fade = tint;
        if matches!(p.kind,Kind::Foam) {t.rotation=Quat::from_rotation_arc(Vec3::Z,world::local(p.position+p.position.normalize(),&f)-world::local(p.position,&f));}
        let unlit = matches!(p.kind, Kind::Smoke | Kind::Spray | Kind::Foam);
        let base_color = match p.kind {
            Kind::Fragment => Color::srgba(0.85, 0.85, 0.85, fade),
            Kind::Smoke => {
                let ash = ((p.age as f32 * 0.035).min(0.12) * 256.0).round() / 256.0;
                Color::srgba(0.055 + ash, 0.06 + ash, 0.065 + ash, fade * 0.65)
            }
            Kind::Flame => Color::srgba(1.0, 0.35, 0.05, fade),
            Kind::Flash => Color::srgba(1.0, 0.8, 0.4, fade),
            Kind::Spark => Color::srgba(1.0, 0.7, 0.18, fade),
            Kind::Spray => Color::srgba(0.78,0.91,1.,fade*0.35),
            Kind::Foam => Color::srgba(0.9,0.97,1.,fade*0.65),
        };
        let emission = match p.kind {
            Kind::Flame => 650.0,
            Kind::Flash => 4000.0 * fade.powi(2),
            Kind::Spark => 3200.0 * fade,
            _ => 0.0,
        };
        let emissive = LinearRgba::new(
            emission * fade,
            emission * 0.25 * fade,
            emission * 0.02 * fade,
            1.0,
        );
        if mats.get(&material.0).is_some_and(|m| m.unlit != unlit
            || m.base_color != base_color || m.emissive != emissive)
            && let Some(m) = mats.get_mut(&material.0)
        {
            m.unlit = unlit;
            m.base_color = base_color;
            m.emissive = emissive;
        }
    }
}

fn draw_diagram(data: &mut [u8], d: &AircraftDamage, integrity: f64, elapsed: f64) {
    if d.aircraft!=crate::aircraft::Aircraft::Gripen {draw_fleet_diagram(data,d,integrity,elapsed);return;}
    data.fill(0);
    polygon(
        data,
        &[
            (110., 2.),
            (119., 37.),
            (121., 92.),
            (99., 92.),
            (101., 37.),
        ],
        color(integrity, SurfaceState::Operational),
    );
    polygon(
        data,
        &[(103., 31.), (25., 81.), (101., 77.)],
        color(d.wings[0], SurfaceState::Operational),
    );
    polygon(
        data,
        &[(117., 31.), (195., 81.), (119., 77.)],
        color(d.wings[1], SurfaceState::Operational),
    );
    for side in 0..2 {
        let mirror = |points: &[(f32, f32)]| {
            points
                .iter()
                .map(|(x, y)| (if side == 0 { *x } else { 220. - *x }, *y))
                .collect::<Vec<_>>()
        };
        for (offset, p) in [
            (0, vec![(102., 24.), (70., 34.), (101., 36.)]),
            (1, vec![(76., 72.), (100., 69.), (101., 78.), (76., 79.)]),
            (2, vec![(27., 78.), (73., 73.), (73., 80.), (27., 84.)]),
            (3, vec![(100., 63.), (105., 63.), (105., 77.), (100., 77.)]),
        ] {
            let i = side * 4 + offset;
            polygon(data, &mirror(&p), color(d.surfaces[i], d.surface_state[i]));
        }
    }
    for (p, h) in [
        (
            vec![(106., 7.), (114., 7.), (116., 18.), (104., 18.)],
            d.radar,
        ),
        (
            vec![(105., 20.), (115., 20.), (115., 31.), (105., 31.)],
            d.avionics,
        ),
        (
            vec![(104., 37.), (116., 37.), (116., 51.), (104., 51.)],
            (1.0 - d.fuel_leak * 0.2).max(0.0),
        ),
        (
            vec![(106., 53.), (114., 53.), (114., 61.), (106., 61.)],
            d.hydraulics,
        ),
        (
            vec![(104., 72.), (116., 72.), (116., 90.), (104., 90.)],
            d.engine,
        ),
    ] {
        polygon(data, &p, color(h, SurfaceState::Operational));
    }
    polygon(
        data,
        &[(108., 63.), (112., 63.), (112., 71.), (108., 71.)],
        color(d.surfaces[8], d.surface_state[8]),
    );
    if d.fire > 0.05 {
        let pulse = (elapsed * 9.0).sin() as f32 * 2.0;
        polygon(
            data,
            &[
                (124., 85.),
                (126., 68. + pulse),
                (132., 77.),
                (134., 62. - pulse),
                (139., 84.),
            ],
            [255, 110, 20, 255],
        );
    }
}
fn draw_fleet_diagram(data:&mut[u8],d:&AircraftDamage,integrity:f64,elapsed:f64){
    data.fill(0);let a=d.aircraft;let s=a.spec();let scale=(85./s.length).min(170./s.span)as f32;
    let project=|p:Vec3|(110.+p.x*scale,48.+p.z*scale);
    let sections=crate::aircraft::visual::sections(a);let mut body:Vec<_>=sections.iter().map(|(z,w,_,_)|project(Vec3::new(-*w as f32,0.,*z as f32))).collect();body.extend(sections.iter().rev().map(|(z,w,_,_)|project(Vec3::new(*w as f32,0.,*z as f32))));polygon(data,&body,color(integrity,SurfaceState::Operational));
    for(side,h)in [(-1.,d.wings[0]),(1.,d.wings[1])]{polygon(data,&crate::aircraft::visual::wing(a,side).map(project),color(h,SurfaceState::Operational));}
    for(i,p)in a.panels().iter().enumerate().filter(|(_,p)|p.area>0.){let(poly,_)=crate::aircraft::visual::panel_polygon(*p);let mut q=poly.map(project);if p.normal==DVec3::X{let at=project(p.center.as_vec3());q=[(at.0-1.5,at.1-6.),(at.0+1.5,at.1-6.),(at.0+1.5,at.1+6.),(at.0-1.5,at.1+6.)];}polygon(data,&q,color(d.surfaces[i],d.surface_state[i]));}
    let box_at=|x:f32,y:f32,w:f32,h:f32|[(x-w,y-h),(x+w,y-h),(x+w,y+h),(x-w,y+h)];
    for(i,p)in a.engine_positions().iter().enumerate().take(s.engines){let(x,y)=project(p.as_vec3());polygon(data,&box_at(x,y,3.,6.),color(d.engines[i],SurfaceState::Operational));}
    for(z,h,w)in [(-s.length*0.40,d.radar,2.),(-s.length*0.25,d.avionics,2.),(0.,d.hydraulics,3.),(s.length*0.05,1./(1.+d.fuel_leak),4.)]{let(x,y)=project(Vec3::new(0.,0.,z as f32));polygon(data,&box_at(x,y,w,3.),color(h,SurfaceState::Operational));}
    if d.fire>0.05{let p=(elapsed*9.).sin()as f32*2.;polygon(data,&[(133.,88.),(131.,73.+p),(138.,81.),(141.,70.-p),(145.,89.)],[255,110,20,255]);}
}
fn diagram_label(d: &AircraftDamage, integrity: f64, destroyed: bool) -> String {
    let names = [
        "L CANARD",
        "L INNER ELEVON",
        "L OUTER ELEVON",
        "L AIRBRAKE",
        "R CANARD",
        "R INNER ELEVON",
        "R OUTER ELEVON",
        "R AIRBRAKE",
        "RUDDER",
    ];
    let mut warnings = Vec::new();
    if destroyed {
        warnings.push("AIRFRAME DESTROYED".into());
    }
    for i in 0..crate::aircraft::count(d.aircraft) {
        let name=if d.aircraft==crate::aircraft::Aircraft::Gripen{names[i].to_string()}else{let p=d.aircraft.panels()[i];format!("{} {}",if p.center.x<0.{"L"}else if p.center.x>0.{"R"}else{""},p.name)};
        match d.surface_state[i] {
            SurfaceState::Jammed(_) => warnings.push(format!("{} JAM", name)),
            SurfaceState::Detached => warnings.push(format!("{} LOST", name)),
            _ => {}
        }
    }
    if d.aircraft!=crate::aircraft::Aircraft::Gripen {for i in 0..d.aircraft.spec().engines{if d.engines[i]<0.15{warnings.push(format!("ENGINE {} FAILED",i+1));}if d.aircraft==crate::aircraft::Aircraft::F22 && d.nozzles[i]<0.15{warnings.push(format!("VECTOR {} JAM",i+1));}}if d.aircraft==crate::aircraft::Aircraft::F35B&&d.lift_fan<0.15{warnings.push("LIFT FAN FAILED".into());}}
    for (h, label) in [
        (d.engine, "ENGINE FAILED"),
        (d.hydraulics, "HYDRAULICS FAILED"),
        (d.radar, "RADAR FAILED"),
        (d.avionics, "AVIONICS FAILED"),
    ] {
        if h < 0.15 {
            warnings.push(label.into());
        }
    }
    if d.fire > 0.05 {
        warnings.push("FIRE".into());
    }
    if d.fuel_leak > 0.0 {
        warnings.push(format!("FUEL LEAK {:.1} kg/s", d.fuel_leak));
    }
    let extra = warnings.len().saturating_sub(3);
    let mut text = warnings.into_iter().take(3).collect::<Vec<_>>().join(" · ");
    if extra > 0 {
        text.push_str(&format!(" +{extra}"));
    }
    if text.is_empty() {
        text = format!("HULL {:.0}% / SYSTEMS NORMAL", integrity * 100.0);
    }
    text
}
