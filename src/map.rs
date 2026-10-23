use crate::{FlightCamera, ViewState, orbit::Orbit, physics::*, world::local};
use bevy::{math::DVec3, prelude::*};
#[path = "maneuver_editor.rs"]
mod editor;
pub use editor::draw as editor_draw;

/// Dim at the beginning of a prediction, bright at its far-future end.
fn trajectory_color(color: Color, progress: f64) -> Color {
    let c = color.to_srgba();
    let fade = progress.clamp(0.0, 1.0).powf(1.4) as f32;
    Color::srgba(c.red, c.green, c.blue, 0.10 + 0.90 * fade)
}

fn current_anomaly(orbit: Orbit, relative: DVec3) -> f64 {
    if orbit.eccentricity <= 1e-6 {
        return 0.0;
    }
    let x = orbit.eccentricity_vector.normalize();
    relative
        .dot(orbit.angular_momentum.normalize().cross(x))
        .atan2(relative.dot(x))
}
#[derive(Resource, Default)]
pub struct Planner {
    pub node: Option<Maneuver>,
    drag: Option<u8>,
    last: Vec2,
    pub capture: bool,
    generation: u64,
    body: crate::space::Body,
    step: usize,
    time_step: usize,
    message: String,
}
impl Planner {
    pub fn capture_context(&mut self, f: &Flight, v: &ViewState) {
        self.body = f.space.body;
        self.generation = v.generation;
    }
}
#[derive(Clone, Copy)]
pub struct Maneuver {
    pub anomaly: f64,
    pub dv: DVec3,
    pub reference: DVec3,
    pub epoch: f64,
}
impl Maneuver {
    pub fn set_eta(&mut self, f: &Flight, seconds: f64) -> bool {
        let o = Orbit::from_flight(f);
        let Some(period) = o.period else {
            return false;
        };
        let (r, v) = crate::space::relative(f);
        let Some((p, _)) = crate::space::propagate(r, v, o.mu, seconds.rem_euclid(period)) else {
            return false;
        };
        let normal = o.angular_momentum.normalize();
        let reference = self.reference_at(f);
        let x = (reference - normal * reference.dot(normal)).normalize();
        self.anomaly = p.dot(normal.cross(x)).atan2(p.dot(x));
        true
    }
    fn reference_at(&self, f: &Flight) -> DVec3 {
        let omega = rotation();
        bevy::math::DQuat::from_axis_angle(
            omega.normalize(),
            -omega.length() * (f.elapsed - self.epoch),
        ) * self.reference
    }
    fn point(&self, f: &Flight, anomaly: f64) -> Option<DVec3> {
        let o = Orbit::from_flight(f);
        let n = o.angular_momentum.normalize_or_zero();
        let reference = self.reference_at(f);
        let x = (reference - n * reference.dot(n)).normalize_or_zero();
        let direction = x * anomaly.cos() + n.cross(x) * anomaly.sin();
        let denominator = 1.0 + o.eccentricity_vector.dot(direction);
        (denominator > 0.05 && direction != DVec3::ZERO).then_some(
            crate::space::center(f.space.body, f.elapsed)
                + direction * o.semi_latus_rectum / denominator,
        )
    }
    pub fn states(&self, f: &Flight) -> Option<(DVec3, DVec3, Orbit)> {
        let orbit = Orbit::from_flight(f);
        let r = self.point(f, self.anomaly)?;
        let relative = r - crate::space::center(f.space.body, f.elapsed);
        let radial = relative.normalize();
        let normal = orbit.angular_momentum.normalize();
        let tangent = normal.cross(radial);
        let h = orbit.angular_momentum.length();
        if h < 1.0 {
            return None;
        }
        let velocity = radial * (-orbit.mu / h * orbit.eccentricity_vector.dot(tangent))
            + tangent * (h / relative.length());
        let impulse = tangent * self.dv.x + radial * self.dv.y + normal * self.dv.z;
        Some((
            r,
            velocity + impulse,
            Orbit::from_state_about(relative, velocity + impulse, orbit.mu, orbit.radius),
        ))
    }
    pub fn eta(&self, f: &Flight) -> Option<f64> {
        let o = Orbit::from_flight(f);
        let period = o.period?;
        let x = if o.eccentricity > 1e-6 {
            o.eccentricity_vector.normalize()
        } else {
            self.reference_at(f).normalize()
        };
        let y = o.angular_momentum.normalize().cross(x);
        let pos = crate::space::relative(f).0;
        let node_relative =
            self.point(f, self.anomaly)? - crate::space::center(f.space.body, f.elapsed);
        let anomaly = pos.dot(y).atan2(pos.dot(x));
        let mean = |nu: f64| {
            let e = o.eccentricity;
            let eccentric = 2.0
                * ((1.0 - e).sqrt() * (nu * 0.5).sin()).atan2((1.0 + e).sqrt() * (nu * 0.5).cos());
            eccentric - e * eccentric.sin()
        };
        Some(
            (mean(node_relative.dot(y).atan2(node_relative.dot(x))) - mean(anomaly))
                .rem_euclid(std::f64::consts::TAU)
                * period
                / std::f64::consts::TAU,
        )
    }
}
#[derive(Component)]
pub struct Root;
#[derive(Component)]
pub struct Readout;
#[derive(Component)]
pub struct Action(u8);
#[derive(Component)]
pub struct Marker(u8);
pub struct Forecast {
    at: f32,
    epoch: f64,
    key: [u64; 4],
    state:[u64;18],
    result: crate::space::navigation::Prediction,
}
fn prediction_state(f:&Flight,node:Option<Maneuver>,generation:u64)->[u64;18]{
    let mut key=[0;18];key[0]=generation;key[1]=f.space.body as u64;
    key[2..5].copy_from_slice(&f.position.to_array().map(f64::to_bits));
    key[5..8].copy_from_slice(&f.velocity.to_array().map(f64::to_bits));key[8]=f.elapsed.to_bits();
    if let Some(n)=node{key[9]=1;key[10]=n.anomaly.to_bits();key[11..14].copy_from_slice(&n.dv.to_array().map(f64::to_bits));key[14..17].copy_from_slice(&n.reference.to_array().map(f64::to_bits));key[17]=n.epoch.to_bits();}
    key
}
pub fn setup(mut c: Commands) {
    editor::setup(&mut c);
    for (id, label, color) in [
        (0, "NODE", Color::srgb(1.0, 0.65, 0.2)),
        (1, "P +", Color::srgb(0.25, 1.0, 0.4)),
        (2, "R +", Color::srgb(0.3, 0.8, 1.0)),
        (3, "N +", Color::srgb(0.8, 0.5, 1.0)),
        (4, "AP", Color::srgb(0.4, 0.85, 1.0)),
        (5, "PE", Color::srgb(0.4, 0.85, 1.0)),
        (6, "SHIP", Color::srgb(0.4, 1.0, 0.7)),
        (7, "MOON", Color::srgb(0.95, 0.9, 0.75)),
        (8, "EARTH", Color::srgb(0.4, 0.7, 1.)),
        (9, "P −", Color::srgb(0.25, 1., 0.4)),
        (10, "R −", Color::srgb(0.3, 0.8, 1.)),
        (11, "N −", Color::srgb(0.8, 0.5, 1.)),
    ] {
        c.spawn((
            Button,
            Marker(id),
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                width: px(52),
                height: px(26),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.035, 0.06, 0.88)),
            ZIndex(43),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(label),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(color),
            ));
        });
    }
}
pub fn controls(
    mut planner: ResMut<Planner>,
    mut f: ResMut<Flight>,
    mut view: ResMut<ViewState>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<FlightCamera>>,
    buttons: Query<(&Action, &Interaction), Changed<Interaction>>,
    handles: Query<(&Marker, &Interaction)>,
    all_buttons: Query<&Interaction, With<Button>>,
    mut warp: Option<ResMut<crate::warp::Warp>>,
) {
    if planner.generation != view.generation || planner.body != f.space.body {
        planner.node = None;
        planner.drag = None;
        planner.generation = view.generation;
        planner.body = f.space.body;
    }
    if !view.map || f.vehicle != Vehicle::Spacecraft {
        planner.drag = None;
        planner.capture = false;
        return;
    }
    for (tag, i) in &buttons {
        if *i == Interaction::Pressed {
            match tag.0 {
                0 => {
                    let o = Orbit::from_flight(&f);
                    let current = if o.eccentricity > 1e-6 {
                        let x = o.eccentricity_vector.normalize();
                        let y = o.angular_momentum.normalize().cross(x);
                        let r = crate::space::relative(&f).0;
                        r.dot(y).atan2(r.dot(x))
                    } else {
                        0.0
                    };
                    planner.node = Some(Maneuver {
                        anomaly: current + 0.4,
                        dv: DVec3::ZERO,
                        reference: if o.eccentricity > 1e-6 {
                            o.eccentricity_vector.normalize()
                        } else {
                            crate::space::relative(&f).0.normalize()
                        },
                        epoch: f.elapsed,
                    });
                }
                1 => planner.node = None,
                2 => {
                    view.map_zoom = 0.5;
                    view.map_focus_ship = true;
                    view.map_target = 0;
                }
                3 => {
                    view.map_zoom = 1.0;
                    view.map_focus_ship = false;
                    view.map_target = 0;
                }
                4 | 5 => {
                    view.map_zoom = 1.;
                    view.map_focus_ship = false;
                    view.map_target = if tag.0 == 4 { 2 } else { 3 };
                }
                6 => {
                    planner.node = crate::space::navigation::moon_transfer(&f);
                }
                34 => {
                    if let Some(warp) = warp.as_deref_mut() {
                        warp_to_maneuver(&mut planner, &mut f, warp);
                    }
                }
                _ => {
                    editor::apply(tag.0, &mut planner, &mut f);
                }
            }
        }
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if mouse.just_pressed(MouseButton::Left) {
        for (marker, i) in &handles {
            if (marker.0 < 4 || marker.0 >= 9) && *i != Interaction::None {
                planner.drag = Some(marker.0);
                planner.last = cursor;
            }
            if *i == Interaction::Pressed && (7..=8).contains(&marker.0) {
                view.map_target = if marker.0 == 7 { 2 } else { 1 };
                view.map_focus_ship = false;
                view.map_zoom = 1.;
            }
        }
        if planner.drag.is_none()
            && all_buttons.iter().all(|i| *i == Interaction::None)
            && cursor.x > 350.
            && cursor.x < window.width() - 370.
            && cursor.y > 210.
        {
            if let Ok((cam, global)) = camera.single() {
                let o = Orbit::from_flight(&f);
                let reference = if o.eccentricity > 1e-6 {
                    o.eccentricity_vector.normalize()
                } else {
                    crate::space::relative(&f).0.normalize()
                };
                let candidate = Maneuver {
                    anomaly: 0.,
                    dv: DVec3::ZERO,
                    reference,
                    epoch: f.elapsed,
                };
                if let Some(anomaly) = nearest_orbit(&candidate, &f, cam, global, cursor, 14. * 14.)
                {
                    planner.node = Some(Maneuver {
                        anomaly,
                        ..candidate
                    });
                    planner.drag = Some(0);
                    planner.last = cursor;
                }
            }
        }
    }
    if !mouse.pressed(MouseButton::Left) {
        planner.drag = None;
    }
    planner.capture = planner.drag.is_some();
    if planner.capture {
        view.left_camera_drag = false;
    }
    let delta = cursor - planner.last;
    planner.last = cursor;
    if let Some(axis) = planner.drag
        && let Some(mut node) = planner.node
    {
        if axis == 0 {
            if let Ok((cam, global)) = camera.single() {
                if let Some(anomaly) = nearest_orbit(&node, &f, cam, global, cursor, f32::INFINITY)
                {
                    node.anomaly = anomaly;
                }
            }
        } else {
            if let Some((component, sign)) = handle_axis(axis) {
                let projected = delta.dot(handle_offset(axis).normalize());
                node.dv[component] = (node.dv[component]
                    + projected as f64 * sign * 2. * editor::dv_step(&planner))
                .clamp(-5000., 5000.);
            }
        }
        planner.node = Some(node);
    }
}
fn handle_axis(id: u8) -> Option<(usize, f64)> {
    match id {
        1..=3 => Some(((id - 1) as usize, 1.)),
        9..=11 => Some(((id - 9) as usize, -1.)),
        _ => None,
    }
}
fn handle_offset(id: u8) -> Vec2 {
    match id {
        1 => Vec2::new(90., 0.),
        9 => Vec2::new(-90., 0.),
        2 => Vec2::new(0., -70.),
        10 => Vec2::new(0., 70.),
        3 => Vec2::new(76., -62.),
        11 => Vec2::new(-76., 62.),
        _ => Vec2::ZERO,
    }
}
fn nearest_orbit(
    node: &Maneuver,
    f: &Flight,
    cam: &Camera,
    global: &GlobalTransform,
    cursor: Vec2,
    limit: f32,
) -> Option<f64> {
    let mut best = limit;
    let mut result = None;
    let o = Orbit::from_flight(f);
    let normal = o.angular_momentum.normalize_or_zero();
    let reference = node.reference_at(f);
    let x = (reference - normal * reference.dot(normal)).normalize_or_zero();
    let y = normal.cross(x);
    let center = crate::space::center(f.space.body, f.elapsed);
    let basis = crate::world::frame(f).conjugate();
    let origin = crate::world::render_origin(f);
    for i in 0..720 {
        let anomaly = i as f64 / 720. * std::f64::consts::TAU;
        let direction = x * anomaly.cos() + y * anomaly.sin();
        let denominator = 1. + o.eccentricity_vector.dot(direction);
        if denominator > 0.05
            && let Ok(pixel) = cam.world_to_viewport(
                global,
                (basis * (center + direction * (o.semi_latus_rectum / denominator) - origin))
                    .as_vec3(),
            )
        {
            let distance = pixel.distance_squared(cursor);
            if distance < best {
                best = distance;
                result = Some(anomaly);
            }
        }
    }
    result
}
#[derive(Default)]
pub(crate) struct MapLines {
    state: Option<[u64;18]>,
    orbit: Vec<DVec3>,
    moon: Vec<DVec3>,
    moon_epoch: Option<u64>,
}
pub fn draw(
    f: Res<Flight>,
    view: Res<ViewState>,
    planner: Res<Planner>,
    mut gizmos: Gizmos,
    camera: Query<(&Camera, &GlobalTransform), With<FlightCamera>>,
    mut roots: Query<&mut Visibility, (With<Root>, Without<Marker>)>,
    mut text: Query<&mut Text, With<Readout>>,
    mut markers: Query<(&Marker, &mut Node, &mut Visibility), Without<Root>>,
    time: Res<Time>,
    mut forecast: Local<Option<Forecast>>,
    mut lines: Local<MapLines>,
) {
    for mut v in &mut roots {
        v.set_if_neq(if view.map && f.vehicle == Vehicle::Spacecraft {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    if !view.map || f.vehicle != Vehicle::Spacecraft {
        for (_, _, mut v) in &mut markers {
            v.set_if_neq(Visibility::Hidden);
        }
        return;
    }
    let orbit = Orbit::from_flight(&f);
    let center = crate::space::center(f.space.body, f.elapsed);
    let relative = f.position - center;
    let color = Color::srgb(0.2, 0.72, 1.0);
    let anomaly = current_anomaly(orbit, relative);
    let line_state=prediction_state(&f,None,view.generation);
    if lines.state!=Some(line_state) {
        lines.orbit.clear();
        lines.orbit.extend((0..=512).map_while(|i|orbit.point(anomaly+i as f64/512.*std::f64::consts::TAU,relative).map(|p|p+center)));
        lines.state=Some(line_state);
    }
    gizmos.linestrip_gradient(lines.orbit.iter().enumerate().map(|(i,p)|(local(*p,&f),trajectory_color(color,i as f64/512.))));
    let scale = if view.map_target == 3 {
        crate::space::MOON_DISTANCE as f32 * 0.004
    } else {
        f.space.body.radius() as f32 * 0.006
    } * view.map_zoom;
    let (mc, _, _) = crate::space::moon(f.elapsed);
    let moon_focus = view.map_target == 2
        || (f.space.body == crate::space::Body::Moon
            && view.map_target != 1
            && view.map_target != 3);
    if !moon_focus {
        if lines.moon_epoch!=Some(f.elapsed.to_bits()) {
            lines.moon.clear();
            lines.moon.extend((0..=256).map(|i|crate::space::earth_rotation(f.elapsed).conjugate()*crate::space::moon_inertial(f.elapsed+i as f64/256.*crate::space::MOON_PERIOD).0));
            lines.moon_epoch=Some(f.elapsed.to_bits());
        }
        gizmos.linestrip(
            lines.moon.iter().map(|p|local(*p,&f)),
            Color::srgba(0.6, 0.65, 0.7, 0.5),
        );
        gizmos.sphere(
            Isometry3d::from_translation(local(mc, &f)),
            crate::space::MOON_SOI as f32,
            Color::srgba(0.7, 0.7, 0.75, 0.12),
        );
    }
    gizmos.sphere(
        Isometry3d::from_translation(local(f.position, &f)),
        scale,
        Color::srgb(0.4, 1.0, 0.7),
    );
    let mut node_position = None;
    if let Some(node) = planner.node
        && let Some((p, _, _)) = node.states(&f)
    {
        node_position = Some(p);
        gizmos.sphere(
            Isometry3d::from_translation(local(p, &f)),
            scale * 1.1,
            Color::srgb(1.0, 0.65, 0.2),
        );
    }
    let km = |value: Option<f64>| {
        value
            .map(|x| format!("{:.1} km", x / 1000.0))
            .unwrap_or_else(|| "ESCAPE".into())
    };
    let mut navigation = String::new();
    let node_key = planner.node.map_or([0; 3], |n| {
        [
            n.dv.x.to_bits() ^ n.anomaly.to_bits(),
            n.dv.y.to_bits(),
            n.dv.z.to_bits(),
        ]
    });
    let key = [
        view.generation ^ f.space.body as u64,
        node_key[0],
        node_key[1],
        node_key[2],
    ];
    let state=prediction_state(&f,planner.node,view.generation);
    if forecast.as_ref().is_none_or(|c|c.key!=key || (c.state!=state && (f.paused||time.elapsed_secs()-c.at>0.2)))
    {
        let mut result=forecast.take().map(|c|c.result).unwrap_or_default();
        crate::space::navigation::predict_into(&f,planner.node,&mut result);
        *forecast = Some(Forecast {
            at: time.elapsed_secs(),
            epoch: f.elapsed,
            key,
            state,
            result,
        });
    }
    if let Some(cache) = forecast.as_ref() {
        let rotation = crate::space::earth_rotation(f.elapsed).conjugate()
            * crate::space::earth_rotation(cache.epoch);
        let start = cache.result.points.first().map_or(f.elapsed, |p| p.1);
        let end = cache.result.points.last().map_or(start + 1.0, |p| p.1);
        // Don't paint several overlapping revolutions over the direction fade.
        let forecast_period = planner.node.map_or(orbit.period, |node| {
            node.states(&f).and_then(|(_, _, o)| o.period)
        });
        let end = if cache.result.encounters.is_empty() {
            forecast_period.map_or(end, |period| end.min(start + period))
        } else {
            end
        };
        let forecast_color = if planner.node.is_some() {
            Color::srgb(1., 0.65, 0.25)
        } else {
            Color::srgb(0.35, 0.9, 0.7)
        };
        gizmos.linestrip_gradient(
            cache
                .result
                .points
                .iter()
                .take_while(|p| p.1 <= end)
                .filter_map(|point| {
                    crate::space::navigation::plot_point(*point, cache.epoch, f.elapsed, moon_focus)
                        .map(|p| {
                            (
                                local(p, &f),
                                trajectory_color(
                                    forecast_color,
                                    (point.1 - start) / (end - start).max(1.0),
                                ),
                            )
                        })
                }),
        );
        for (body, eta, p) in cache.result.encounters.iter().take(2) {
            gizmos.sphere(
                Isometry3d::from_translation(local(rotation * *p, &f)),
                scale * 1.3,
                Color::srgb(1., 0.75, 0.2),
            );
            navigation = format!(
                "\n{} ENCOUNTER T − {:.1} h",
                body.label(),
                (eta + cache.epoch - f.elapsed) / 3600.
            );
        }
        if navigation.is_empty() && f.space.body == crate::space::Body::Earth {
            navigation = format!(
                "\nLUNAR CLOSEST ≈ {:.0} km",
                cache.result.closest_moon / 1000.
            );
        }
        if let Some((body, eta)) = cache.result.impact {
            navigation = format!(
                "\n{} {} IN {:.0} s",
                body.label(),
                if body == crate::space::Body::Earth {
                    "ENTRY"
                } else {
                    "SURFACE"
                },
                eta + cache.epoch - f.elapsed
            );
        }
    }
    for mut t in &mut text {
        let summary = format!(
            "ORBIT MAP · NORTH UP\n{} / ALT {:.1} km\nAP {} / PE {:.1} km\nSPEED {:.0} m/s / PERIOD {}\nAVAILABLE Δv {:.0} m/s{}",
            f.space.body.label(),
            f.altitude() / 1000.,
            km(orbit.apoapsis),
            orbit.periapsis / 1000.,
            orbit.speed,
            orbit
                .period
                .map(|p| format!("{:.1}m", p / 60.))
                .unwrap_or("ESCAPE".into()),
            crate::space::delta_v(&f),
            navigation
        );
        if t.0 != summary {
            t.0 = summary;
        }
    }
    let Ok((cam, global)) = camera.single() else {
        return;
    };
    for (marker, mut style, mut vis) in &mut markers {
        if view.map_target == 3 && (4..=5).contains(&marker.0) {
            *vis = Visibility::Hidden;
            continue;
        }
        let p = match marker.0 {
            0..=3 | 9..=11 => node_position,
            4 => {
                if orbit.apoapsis.is_some() {
                    orbit
                        .point(std::f64::consts::PI, relative)
                        .map(|p| p + center)
                } else {
                    None
                }
            }
            5 => orbit.point(0.0, relative).map(|p| p + center),
            7 => Some(mc),
            8 => Some(DVec3::ZERO),
            _ => Some(f.position),
        };
        if let Some(p) = p
            && let Ok(mut pixel) = cam.world_to_viewport(global, local(p, &f))
        {
            pixel += match marker.0 {
                1..=3 | 9..=11 => handle_offset(marker.0),
                7 => Vec2::new(0., -25.),
                8 => Vec2::new(0., 25.),
                6 if view.map_target == 3 => Vec2::new(0., -25.),
                _ => Vec2::ZERO,
            };
            let size = cam
                .logical_viewport_size()
                .unwrap_or(Vec2::new(1440., 900.));
            if pixel.x < 360. || pixel.x > size.x - 380. || pixel.y < 215. || pixel.y > size.y - 26.
            {
                *vis = Visibility::Hidden;
                continue;
            }
            style.left = px(pixel.x - 26.0);
            style.top = px(pixel.y - 13.0);
            *vis = Visibility::Inherited;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}
fn maneuver_stop(node: Maneuver, f: &Flight) -> Option<f64> {
    let eta = node.eta(f)?;
    let (thrust, isp) = f.space.stage.engine();
    if thrust <= 0.0 || node.dv.length() <= 0.001 {
        return None;
    }
    let fuel = f.mass() * (1.0 - (-node.dv.length() / (isp * crate::space::G0)).exp());
    let duration = fuel / (thrust / (isp * crate::space::G0));
    Some(f.elapsed + (eta - duration * 0.5 - 15.0).max(0.0))
}

fn warp_to_maneuver(p: &mut Planner, f: &mut Flight, warp: &mut crate::warp::Warp) {
    let Some(stop) = p.node.and_then(|node| maneuver_stop(node, f)) else {
        p.message = "Select a maneuver with nonzero Δv first".into();
        return;
    };
    if stop <= f.elapsed + 1.0 {
        p.message = "Already at the burn preparation window".into();
        return;
    }
    let previous = warp.selected;
    warp.selected = warp.rates.len().saturating_sub(1);
    if warp.allowed(f) <= 4.0 {
        warp.selected = previous;
        p.message = "Warp requires a safe orbital coast: throttle off and rotation settled".into();
        return;
    }
    f.space.burn_stop = Some(stop);
    f.paused = false;
    p.message = "Warping to 15 seconds before burn start".into();
    warp.message = p.message.clone();
}

pub fn guidance(planner: Res<Planner>, mut f: ResMut<Flight>) {
    if f.vehicle != Vehicle::Spacecraft {
        return;
    }
    f.space.burn_stop = planner.node.and_then(|n| maneuver_stop(n, &f));
    f.space.burn_direction = if let Some(node) = planner.node {
        if let Some((p, _, _)) = node.states(&f) {
            let radial = (p - crate::space::center(f.space.body, f.elapsed)).normalize();
            let normal = Orbit::from_flight(&f).angular_momentum.normalize();
            normal.cross(radial) * node.dv.x + radial * node.dv.y + normal * node.dv.z
        } else {
            DVec3::ZERO
        }
    } else {
        DVec3::ZERO
    };
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prediction_cache_key_tracks_epoch_state_node_frame_and_scene() {
        let f=Flight::spacecraft();let key=prediction_state(&f,None,1);
        let mut changed=f.clone();changed.position.x+=0.001;assert_ne!(key,prediction_state(&changed,None,1));
        changed=f.clone();changed.velocity.y+=0.001;assert_ne!(key,prediction_state(&changed,None,1));
        changed=f.clone();changed.elapsed+=0.01;assert_ne!(key,prediction_state(&changed,None,1));
        assert_ne!(key,prediction_state(&f,None,2));
        let node=Maneuver {anomaly:0.4,dv:DVec3::X,reference:DVec3::Y,epoch:f.elapsed};
        let key=prediction_state(&f,Some(node),1);
        for changed in [Maneuver {reference:DVec3::Z,..node},Maneuver {epoch:node.epoch+1.,..node},Maneuver {dv:DVec3::Z,..node},Maneuver {anomaly:0.5,..node}] {
            assert_ne!(key,prediction_state(&f,Some(changed),1));
        }
    }
    #[test]
    fn warp_to_maneuver_arms_existing_burn_stop_and_respects_safe_coast_limits() {
        let mut f = Flight::spacecraft();
        f.paused = true;
        let node = Maneuver {
            anomaly: 0.4,
            dv: DVec3::new(120.0, 0.0, 0.0),
            reference: f.position.normalize(),
            epoch: f.elapsed,
        };
        let mut planner = Planner {
            node: Some(node),
            ..default()
        };
        let mut warp = crate::warp::Warp::default();
        warp_to_maneuver(&mut planner, &mut f, &mut warp);
        assert!(!f.paused);
        assert_eq!(warp.selected, warp.rates.len() - 1);
        let stop = f.space.burn_stop.unwrap();
        assert!(stop > f.elapsed && stop < f.elapsed + node.eta(&f).unwrap() - 15.0);
        f.paused = true;
        f.throttle = 1.0;
        warp.normal();
        warp_to_maneuver(&mut planner, &mut f, &mut warp);
        assert_eq!(warp.rates[warp.selected],1.);
        assert!(f.paused);
        planner.node = None;
        warp_to_maneuver(&mut planner, &mut f, &mut warp);
        assert!(planner.message.contains("Select a maneuver"));
    }
    #[test]
    fn future_orbit_starts_at_ship_and_advances_along_velocity() {
        for id in [3, 9] {
            let f = crate::space::scenario(id);
            let (r, v) = crate::space::relative(&f);
            let orbit = Orbit::from_flight(&f);
            let start = current_anomaly(orbit, r);
            let p0 = orbit.point(start, r).unwrap();
            let p1 = orbit.point(start + 0.001, r).unwrap();
            assert!(p0.distance(r) < 1e-5);
            assert!((p1 - p0).dot(v) > 0.0);
        }
        let base = Color::srgb(0.2, 0.72, 1.0);
        let mut previous = 0.0;
        for i in 0..=100 {
            let c = trajectory_color(base, i as f64 / 100.0).to_srgba();
            assert!(c.alpha >= previous && c.alpha <= 1.0);
            assert_eq!(c.blue, 1.0);
            previous = c.alpha;
        }
        assert_eq!(trajectory_color(base, 0.0).to_srgba().alpha, 0.1);
        assert_eq!(trajectory_color(base, 1.0).to_srgba().alpha, 1.0);
    }
    #[test]
    fn precision_button_updates_node_even_without_a_window_cursor() {
        let mut app = App::new();
        let f = Flight::spacecraft();
        let node = Maneuver {
            anomaly: 0.4,
            dv: DVec3::ZERO,
            reference: f.position.normalize(),
            epoch: f.elapsed,
        };
        app.insert_resource(f)
            .insert_resource(ViewState {
                map: true,
                ..default()
            })
            .insert_resource(Planner {
                node: Some(node),
                ..default()
            })
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(Update, controls);
        app.world_mut()
            .spawn((Button, Action(13), Interaction::Pressed));
        app.update();
        assert_eq!(app.world().resource::<Planner>().node.unwrap().dv.y, 1.);
    }
    #[test]
    fn precise_editor_controls_change_components_time_and_sas_without_thrust() {
        for id in [3, 9] {
            let mut f = crate::space::scenario(id);
            let mut node = Maneuver {
                anomaly: 0.4,
                dv: DVec3::ZERO,
                reference: crate::space::relative(&f).0.normalize(),
                epoch: f.elapsed,
            };
            assert!(node.set_eta(&f, 240.));
            assert!((node.eta(&f).unwrap() - 240.).abs() < 1e-4);
            let mut p = Planner {
                node: Some(node),
                ..default()
            };
            for axis in 0..3 {
                editor::apply(11 + axis * 2, &mut p, &mut f);
            }
            assert_eq!(p.node.unwrap().dv, DVec3::ONE);
            editor::apply(20, &mut p, &mut f);
            editor::apply(11, &mut p, &mut f);
            assert_eq!(p.node.unwrap().dv.x, 11.);
            editor::apply(31, &mut p, &mut f);
            assert!((p.node.unwrap().eta(&f).unwrap() - 300.).abs() < 1e-4);
            editor::apply(30, &mut p, &mut f);
            assert!((p.node.unwrap().eta(&f).unwrap() - 240.).abs() < 1e-4);
            editor::apply(33, &mut p, &mut f);
            assert_eq!(f.sas_mode, 5);
            assert!(f.assist && f.throttle == 0.);
            editor::apply(32, &mut p, &mut f);
            assert_eq!(p.node.unwrap().dv, DVec3::ZERO);
            for _ in 0..1000 {
                editor::apply(15, &mut p, &mut f);
            }
            assert_eq!(p.node.unwrap().dv.z, 5000.);
        }
    }
    #[test]
    fn signed_handles_pair_each_axis_with_opposite_drag_direction() {
        for (plus, minus) in [(1, 9), (2, 10), (3, 11)] {
            let (a, s) = handle_axis(plus).unwrap();
            let (b, t) = handle_axis(minus).unwrap();
            assert_eq!(a, b);
            assert_eq!(s, -t);
            assert!(
                handle_offset(plus)
                    .normalize()
                    .dot(handle_offset(minus).normalize())
                    < -0.999
            );
        }
    }
    #[test]
    fn lunar_node_uses_moon_gravity_and_eta_counts_down() {
        let mut f = crate::space::scenario(9);
        let node = Maneuver {
            anomaly: 0.8,
            dv: DVec3::new(20., 0., 0.),
            reference: crate::space::relative(&f).0.normalize(),
            epoch: f.elapsed,
        };
        let before = node.eta(&f).unwrap();
        let (p, _, post) = node.states(&f).unwrap();
        assert_eq!(post.mu, crate::space::MOON_MU);
        assert!(
            (p.distance(crate::space::moon(f.elapsed).0) - crate::space::MOON_RADIUS - 100000.)
                .abs()
                < 0.01
        );
        crate::warp::coast(&mut f, &mut Environment::default(), 60.);
        assert!((node.eta(&f).unwrap() - (before - 60.)).abs() < 0.01);
    }
    #[test]
    fn node_eta_counts_down_during_coast_instead_of_following_the_ship() {
        let mut f = Flight::spacecraft();
        let node = Maneuver {
            anomaly: 0.5,
            dv: DVec3::ZERO,
            reference: f.position.normalize(),
            epoch: f.elapsed,
        };
        let before = node.eta(&f).unwrap();
        let mut e = Environment::default();
        crate::warp::coast(&mut f, &mut e, 100.0);
        assert!((node.eta(&f).unwrap() - (before - 100.0)).abs() < 0.1);
    }
    #[test]
    fn dragging_a_delta_v_handle_changes_prediction_and_release_ends_drag() {
        let mut app = App::new();
        let f = Flight::spacecraft();
        let node = Maneuver {
            anomaly: 0.4,
            dv: DVec3::ZERO,
            reference: f.position.normalize(),
            epoch: f.elapsed,
        };
        let mut view = ViewState::default();
        view.map = true;
        view.left_camera_drag = true;
        app.add_plugins(MinimalPlugins)
            .insert_resource(f)
            .insert_resource(view)
            .insert_resource(Planner {
                node: Some(node),
                ..default()
            })
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(Update, controls);
        let mut window = Window::default();
        window.set_cursor_position(Some(Vec2::new(400.0, 300.0)));
        let window = app.world_mut().spawn(window).id();
        app.world_mut().spawn((Marker(1), Interaction::Pressed));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(!app.world().resource::<ViewState>().left_camera_drag);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(440.0, 290.0)));
        app.update();
        assert!((app.world().resource::<Planner>().node.unwrap().dv.x - 80.0).abs() < 1e-6);
        let f = app.world().resource::<Flight>();
        assert!(
            app.world()
                .resource::<Planner>()
                .node
                .unwrap()
                .states(f)
                .unwrap()
                .2
                .apoapsis
                .unwrap()
                > 400000.0
        );
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        assert!(!app.world().resource::<Planner>().capture);
    }
    #[test]
    fn prograde_node_raises_apoapsis_and_zero_node_preserves_orbit() {
        let f = Flight::spacecraft();
        let mut node = Maneuver {
            anomaly: 0.4,
            dv: DVec3::ZERO,
            reference: f.position.normalize(),
            epoch: f.elapsed,
        };
        let zero = node.states(&f).unwrap().2;
        assert!((zero.periapsis - 220000.0).abs() < 0.1);
        node.dv.x = 100.0;
        let post = node.states(&f).unwrap().2;
        assert!(post.apoapsis.unwrap() > 500000.0);
        assert!(node.eta(&f).unwrap() > 0.0);
    }
}
