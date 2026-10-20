use super::*;
use bevy::{
    asset::RenderAssetUsages,
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

#[derive(Resource, Clone)]
pub struct Menu {
    pub formation: usize,
    pub difficulty: Difficulty,
    pub defense: bool,
    pub open: bool,
    pub saved_pause: Option<bool>,
    pub reveal: bool,
    pub(super) center: Vec2,
    pub(super) span: f32,
    pub(super) dirty: bool,
    pub(super) image: Handle<Image>,
    pub(super) last_generation: u64,
}
impl Default for Menu {
    fn default() -> Self {
        Self {
            formation: 1,
            difficulty: Difficulty::Balanced,
            defense: false,
            open: false,
            saved_pause: None,
            reveal: false,
            center: Vec2::new(100000., -50000.),
            span: 300000.,
            dirty: true,
            image: default(),
            last_generation: 0,
        }
    }
}
#[derive(Component)]
pub struct Action(u8);
#[derive(Component)]
pub struct Panel;
#[derive(Component)]
pub struct Readout;
#[derive(Component)]
pub struct MapImage;
#[derive(Component)]
pub struct Backdrop;
#[derive(Component)]
pub struct Symbol(usize, u8);
#[derive(Component)]
pub struct PickTarget(Option<usize>);
#[derive(Component)]
pub struct Status;
fn button(p: &mut ChildSpawnerCommands, id: u8, label: &str) {
    p.spawn((
        Button,
        Action(id),
        Node {
            height: px(30),
            padding: UiRect::horizontal(px(8)),
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.055, 0.15, 0.20)),
    ))
    .with_children(|p| {
        p.spawn((
            Text::new(label),
            TextFont {
                font_size: 13.,
                ..default()
            },
            TextColor(Color::srgb(0.70, 0.93, 0.97)),
        ));
    });
}
pub fn setup(mut c: Commands, mut menu: ResMut<Menu>, mut images: ResMut<Assets<Image>>) {
    super::setup_screen::spawn(&mut c);
    super::cockpit_radar::setup(&mut c);
    c.spawn((
        Backdrop,
        Visibility::Hidden,
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            right: px(0),
            top: px(155),
            bottom: px(0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.009, 0.022, 0.03)),
        GlobalZIndex(58),
    ));
    menu.image = images.add(Image::new_fill(
        Extent3d {
            width: 512,
            height: 512,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[5, 15, 20, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    c.spawn((
        Button,
        Action(0),
        crate::Chrome,
        Node {
            position_type: PositionType::Absolute,
            left: px(580),
            top: px(173),
            width: px(115),
            height: px(32),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.05, 0.18, 0.22)),
        GlobalZIndex(52),
    ))
    .with_children(|p| {
        p.spawn((
            Text::new("BVR SETUP"),
            TextFont {
                font_size: 14.,
                ..default()
            },
            TextColor(Color::srgb(0.65, 1., 0.9)),
        ));
    });
    c.spawn((
        Panel,
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            left: px(20),
            top: px(165),
            width: px(275),
            padding: UiRect::all(px(14)),
            row_gap: px(4),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::srgb(0.018, 0.045, 0.065)),
        GlobalZIndex(70),
    ))
    .with_children(|p| {
        p.spawn((
            Readout,
            Text::new(""),
            TextFont {
                font_size: 13.,
                ..default()
            },
            TextColor(Color::srgb(0.75, 0.91, 0.95)),
        ));
        for (id, label) in [
            (12, "RADAR POWER / STANDBY"),
            (5, "RADAR RANGE / NEXT"),
            (6, "ELEVATION +5°"),
            (7, "ELEVATION −5°"),
            (8, "MODE: SEARCH / TWS / STT"),
            (9, "SENSORS / REVEAL ALL"),
            (10, "CENTER ON PLAYER"),
            (11, "CLOSE PANEL / MAP"),
        ] {
            button(p, id, label);
        }
    });
    c.spawn((
        MapImage,
        ImageNode::new(menu.image.clone()),
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            left: px(310),
            top: px(165),
            width: px(800),
            height: px(600),
            ..default()
        },
        GlobalZIndex(60),
    ));
    for i in 0..77 {
        for kind in 0..3 {
            let mut e = c.spawn((
                Symbol(i, kind),
                PickTarget(None),
                UiTransform::default(),
                Text::new(""),
                TextFont {
                    font_size: 13.,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    display: Display::None,
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.01, 0.025, 0.04, 0.85)),
                GlobalZIndex(65),
            ));
            if kind == 1 {
                e.insert(Button);
            }
        }
    }
    c.spawn((
        Status,
        Text::new(""),
        TextFont {
            font_size: 13.,
            ..default()
        },
        TextColor(Color::srgb(0.6, 1., 0.83)),
        Node {
            position_type: PositionType::Absolute,
            right: px(28),
            top: px(165),
            width: px(420),
            padding: UiRect::all(px(8)),
            ..default()
        },
        GlobalZIndex(55),
        BackgroundColor(Color::srgb(0.018, 0.045, 0.065)),
    ));
}
fn rect(w: &Window) -> (Vec2, Vec2) {
    (
        Vec2::new(310., 165.),
        Vec2::new((w.width() - 335.).max(200.), (w.height() - 200.).max(200.)),
    )
}
fn to_screen(p: DVec3, m: &Menu, origin: Vec2, size: Vec2) -> Vec2 {
    origin + ((Vec2::new(p.x as f32, p.z as f32) - m.center) / m.span + Vec2::splat(0.5)) * size
}
pub fn controls(
    contacts: Query<(&Interaction, &PickTarget), Changed<Interaction>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    windows: Query<&Window>,
    mut buttons: Query<(&Action, &Interaction, &mut BackgroundColor), Changed<Interaction>>,
    mut menu: ResMut<Menu>,
    mut v: ResMut<ViewState>,
    mut f: ResMut<Flight>,
    mut combat: ResMut<Combat>,
    warp: Res<crate::warp::Warp>,
) {
    if warp.editing {
        return;
    }
    for (interaction, pick) in &contacts {
        if *interaction == Interaction::Pressed {
            if let Some(i) = pick.0 {
                combat.selected = i;
                combat.lock = 0.;
            }
        }
    }
    for (a, i, mut color) in &mut buttons {
        color.0 = if *i == Interaction::Hovered {
            Color::srgb(0.10, 0.28, 0.32)
        } else {
            Color::srgb(0.055, 0.15, 0.20)
        };
        if *i != Interaction::Pressed {
            continue;
        }
        match a.0 {
            0 => {
                if menu.open {
                    super::setup_screen::cancel(&mut menu, &mut f);
                } else {
                    menu.saved_pause = Some(f.paused);
                    menu.open = true;
                    f.paused = true;
                    combat.stop_gun();
                }
            }
            1 => menu.formation = (menu.formation + 1) % 4,
            2 => {
                menu.difficulty = match menu.difficulty {
                    Difficulty::Training => Difficulty::Balanced,
                    Difficulty::Balanced => Difficulty::Hard,
                    Difficulty::Hard => Difficulty::Training,
                }
            }
            3 => menu.defense = !menu.defense,
            4 => {
                crate::reset(&mut f, 8, &mut v);
                menu.open = false;
                menu.saved_pause = None;
                menu.dirty = true;
            }
            5 => {
                if let Some(b) = &mut combat.bvr {
                    b.radar.range = (b.radar.range + 1) % 4;
                }
            }
            6 | 7 => {
                if let Some(b) = &mut combat.bvr {
                    b.radar.elevation =
                        (b.radar.elevation + if a.0 == 6 { 5. } else { -5. }).clamp(-45., 45.);
                }
            }
            8 => {
                if let Some(b) = &mut combat.bvr {
                    b.radar.mode = match b.radar.mode {
                        RadarMode::Search => RadarMode::Tws,
                        RadarMode::Tws => RadarMode::Stt,
                        RadarMode::Stt => RadarMode::Search,
                    };
                    combat.lock = 0.;
                }
            }
            9 => menu.reveal = !menu.reveal,
            10 => {
                menu.center = Vec2::new(f.position.x as f32, f.position.z as f32);
                menu.dirty = true;
            }
            11 => {
                v.map = false;
            }
            12 => super::cockpit_radar::command(&mut combat, super::cockpit_radar::Command::Power),
            _ => {}
        }
    }
    if menu.open {
        return;
    }
    if !v.map || f.vehicle != Vehicle::Gripen {
        return;
    }
    let Ok(w) = windows.single() else {
        return;
    };
    let (origin, size) = rect(w);
    if menu.last_generation != v.generation {
        menu.center = Vec2::new(f.position.x as f32, f.position.z as f32 - 50000.);
        menu.last_generation = v.generation;
        menu.dirty = true;
    }
    if let Some(cursor) = w.cursor_position().filter(|p| {
        p.x >= origin.x && p.y >= origin.y && p.x <= origin.x + size.x && p.y <= origin.y + size.y
    }) {
        if scroll.delta.y != 0. {
            menu.span = (menu.span * (-scroll.delta.y * 0.12).exp()).clamp(20000., 600000.);
            menu.dirty = true;
        }
        if mouse.pressed(MouseButton::Right) {
            let span = menu.span;
            menu.center -= motion.delta / size * span;
            menu.dirty |= motion.delta != Vec2::ZERO;
        }
        if mouse.just_pressed(MouseButton::Left) {
            let selected = combat
                .targets
                .iter()
                .enumerate()
                .filter(|(i, t)| {
                    t.alive
                        && combat.bvr.as_ref().is_some_and(|b| {
                            *i < b.enemies
                                && (menu.reveal || b.radar.tracks[*i].valid(combat.clock))
                        })
                })
                .map(|(i, t)| {
                    let p = combat
                        .bvr
                        .as_ref()
                        .filter(|_| !menu.reveal)
                        .map_or(t.position, |b| {
                            b.radar.tracks[i].position
                                + b.radar.tracks[i].velocity
                                    * (combat.clock - b.radar.tracks[i].seen)
                        });
                    (i, to_screen(p, &menu, origin, size).distance(cursor))
                })
                .filter(|(_, d)| *d < 35.)
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((i, _)) = selected {
                combat.selected = i;
                combat.lock = 0.;
            }
        }
    }
    if keys.just_pressed(KeyCode::Enter) {
        if let Some(b) = &mut combat.bvr {
            b.radar.mode = RadarMode::Stt;
        }
    }
}
pub fn draw(
    mut backdrop: Query<&mut Visibility, With<Backdrop>>,
    mut menu: ResMut<Menu>,
    v: Res<ViewState>,
    f: Res<Flight>,
    combat: Res<Combat>,
    windows: Query<&Window>,
    mut images: ResMut<Assets<Image>>,
    mut panel: Query<&mut Node, (With<Panel>, Without<MapImage>, Without<Symbol>)>,
    mut map: Query<&mut Node, (With<MapImage>, Without<Panel>, Without<Symbol>)>,
    mut readout: Query<&mut Text, (With<Readout>, Without<Symbol>, Without<Status>)>,
    mut status: Query<
        (&mut Text, &mut Node),
        (
            With<Status>,
            Without<Symbol>,
            Without<MapImage>,
            Without<Panel>,
        ),
    >,
    mut symbols: Query<
        (
            &Symbol,
            &mut Node,
            &mut Text,
            &mut TextColor,
            &mut UiTransform,
            &mut BackgroundColor,
            &mut PickTarget,
        ),
        (
            Without<Panel>,
            Without<MapImage>,
            Without<Status>,
            Without<Readout>,
        ),
    >,
) {
    let tactical = v.map && f.vehicle == Vehicle::Gripen && !menu.open;
    for mut visible in &mut backdrop {
        *visible = if tactical {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let Ok(w) = windows.single() else {
        return;
    };
    let (origin, size) = rect(w);
    for mut n in &mut panel {
        n.display = if tactical {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut n in &mut map {
        n.display = if tactical {
            Display::Flex
        } else {
            Display::None
        };
        n.width = px(size.x);
        n.height = px(size.y);
    }
    for mut t in readout.iter_mut().filter(|_| tactical) {
        let radar = combat
            .bvr
            .as_ref()
            .map_or("Start a BVR scenario to use radar".into(), |b| {
                format!(
                    "{} / {:.0} km\nSCAN {:+.0}°  EL {:+.0}°\nTRACKS {} / 8 TWS links",
                    b.radar.mode.label(),
                    RANGES[b.radar.range] / 1000.,
                    b.radar.scan,
                    b.radar.elevation,
                    b.radar
                        .tracks
                        .iter()
                        .take(b.enemies)
                        .filter(|t| t.valid(combat.clock))
                        .count()
                )
            });
        t.0 = format!(
            "TACTICAL / LIVE RADAR\n{}\nMAP: {}\nSpan {:.0} km / north up\n\nClick contact: designate\nEnter: single-target lock\nRight-drag: pan\nWheel: zoom\nM: return to flight\n\nScenario configuration is\nunder BVR SETUP.",
            radar,
            if menu.reveal {
                "REVEAL ALL / TRAINING"
            } else {
                "SENSOR CONTACTS"
            },
            menu.span / 1000.
        );
    }
    for (mut t, mut n) in &mut status {
        n.display = if combat.bvr.is_some() && !tactical {
            Display::Flex
        } else {
            Display::None
        };
        if let Some(b) = &combat.bvr {
            let selected = combat
                .targets
                .get(combat.selected)
                .zip(b.radar.tracks.get(combat.selected))
                .filter(|(_, tr)| tr.valid(combat.clock));
            let detail=selected.map_or("NO RADAR TRACK / M FOR SCAN CONTROLS".into(),|(_,tr)|{
                let r=tr.position-f.position;let local=f.orientation.conjugate()*r;
                let az=local.x.atan2(-local.z).to_degrees();let el=local.y.atan2(local.x.hypot(local.z)).to_degrees();
                let contact=Contact{position:tr.position,velocity:tr.velocity,forward:-tr.velocity.normalize_or_zero(),throttle:1.,engine:true,alive:true,decoy:false,strength:1.};
                let(max,nez)=envelope(f.position,f.velocity,contact);
                format!("T{:02} {:.1} km / {:.0} m / AZ {:+.0}° EL {:+.0}°\nRMAX EST {:.0} km · NEZ EST {:.0} km · {}",combat.selected+1,r.length()/1000.,tr.position.length()-RADIUS,az,el,max/1000.,nez/1000.,combat.cue(&f).err().unwrap_or(if combat.lock>=combat.weapon.lock_time(){"SHOOT"}else{"ACQUIRING"}))
            });
            let incoming = combat
                .missiles
                .iter()
                .zip(&b.shots)
                .filter(|(m, s)| {
                    m.alive
                        && !s.friendly
                        && s.target.is_none()
                        && m.position.distance(f.position) < 40000.
                })
                .count();
            let value = format!(
                "BVR {} / {} · {}    M MAP\n{}\nLR-AAM {} · IR {} · INCOMING {} · {}\nFORCE {} vs {} / START {:.0} km",
                b.difficulty.label(),
                if b.defense { "DEFEND" } else { "ELIMINATE" },
                b.radar.mode.label(),
                detail,
                combat.ammo[1],
                combat.ammo[0],
                incoming,
                b.result,
                b.allies + 1,
                b.enemies,
                b.range_at_start / 1000.
            );
            if t.0 != value {
                t.0 = value;
            }
        }
    }
    if !tactical {
        for (_, mut n, _, _, _, _, mut pick) in &mut symbols {
            if n.display != Display::None {
                n.display = Display::None;
            }
            if pick.0.is_some() {
                pick.0 = None;
            }
        }
        return;
    }
    if tactical && menu.dirty {
        if let Some(image) = images.get_mut(&menu.image) {
            if let Some(data) = image.data.as_mut() {
                for y in 0..512 {
                    for x in 0..512 {
                        let xx = menu.center.x + (x as f32 / 511. - 0.5) * menu.span;
                        let zz = menu.center.y + (y as f32 / 511. - 0.5) * menu.span;
                        let h = terrain_height(xx as f64, zz as f64).max(0.);
                        let shade = (h / 3000.).clamp(0., 1.) as f32;
                        let contour = ((h / 250.).fract() < 0.04) as u8;
                        let grid = (x % 64 == 0 || y % 64 == 0) as u8;
                        let index = (y * 512 + x) * 4;
                        data[index] = (18. + shade * 65.) as u8 + contour * 12 + grid * 8;
                        data[index + 1] = (38. + shade * 62.) as u8 + contour * 12 + grid * 8;
                        data[index + 2] = (39. + shade * 50.) as u8 + contour * 12 + grid * 8;
                        data[index + 3] = 255;
                    }
                }
            }
        }
        menu.dirty = false;
    }
    let mut marks: Vec<(DVec3, String, Color)> =
        vec![(f.position, "YOU".into(), Color::srgb(0.4, 1., 0.85))];
    let mut pick_ids = vec![None];
    if let Some(b) = &combat.bvr {
        marks.push((
            b.objective,
            if b.defense {
                "DEFEND / 18 km line"
            } else {
                "HOME SECTOR"
            }
            .into(),
            Color::srgb(0.5, 0.8, 1.),
        ));
        pick_ids.push(None);
        for (i, t) in combat.targets.iter().enumerate().filter(|(_, t)| t.alive) {
            let tr = &b.radar.tracks[i];
            let friend = b.friendly(i);
            if !friend && !menu.reveal && !tr.valid(combat.clock) {
                continue;
            }
            pick_ids.push(if friend { None } else { Some(i) });
            let p = if menu.reveal || friend {
                t.position
            } else {
                tr.position + tr.velocity * (combat.clock - tr.seen)
            };
            let label = format!(
                "{} {}{:02}{}\n{:.1} km / {:.0} m{}",
                if i == combat.selected {
                    "*"
                } else if friend {
                    "+"
                } else {
                    "+"
                },
                if friend {
                    "ALLY "
                } else if menu.reveal || tr.reports >= 2 {
                    "HOSTILE "
                } else {
                    "UNK "
                },
                i + 1,
                if !menu.reveal && combat.clock - tr.seen > 3. {
                    " COAST"
                } else {
                    ""
                },
                f.position.distance(p) / 1000.,
                p.length() - RADIUS,
                if tr.friendly_report && !menu.reveal {
                    " DL"
                } else {
                    ""
                }
            );
            marks.push((
                p,
                label,
                if friend {
                    Color::srgb(0.4, 0.8, 1.)
                } else {
                    Color::srgb(1., 0.55, 0.35)
                },
            ));
        }
        for (m, s) in combat.missiles.iter().zip(&b.shots).filter(|(m, s)| {
            m.alive
                && (menu.reveal
                    || s.friendly
                    || m.gnc.active && m.position.distance(f.position) < 35000.)
        }) {
            let closure = m.gnc.estimated_velocity - m.velocity;
            let r = m.gnc.estimate - m.position;
            let closing = (-closure.dot(r.normalize_or_zero())).max(1.);
            pick_ids.push(None);
            marks.push((
                m.position,
                format!(
                    "MISSILE {}\n{:.0}s / M{:.1}",
                    m.gnc.mode.label(),
                    r.length() / closing,
                    m.velocity.length() / atmosphere(m.position.length() - RADIUS).3
                ),
                if s.friendly {
                    Color::srgb(0.7, 1., 0.7)
                } else {
                    Color::srgb(1., 0.25, 0.3)
                },
            ));
        }
    }
    let mut layout = Vec::new();
    let mut occupied: Vec<Vec2> = Vec::new();
    for (p, label, c) in &marks {
        let anchor = to_screen(*p, &menu, origin, size);
        let inside = anchor.x >= origin.x
            && anchor.y >= origin.y
            && anchor.x < origin.x + size.x
            && anchor.y < origin.y + size.y;
        let mut position = anchor + Vec2::new(10., -8.);
        if inside {
            for attempt in 0..200 {
                let row = attempt % 40;
                let column = attempt / 40;
                let horizontal = match column {
                    0 => 10.,
                    1 => 190.,
                    2 => -170.,
                    3 => 370.,
                    _ => -350.,
                };
                let offset = if row % 2 == 0 {
                    row as f32 / 2.
                } else {
                    -((row + 1) as f32 / 2.)
                } * 36.;
                let test = Vec2::new(
                    (anchor.x + horizontal).clamp(origin.x + 4., origin.x + size.x - 174.),
                    (anchor.y - 8. + offset).clamp(origin.y + 4., origin.y + size.y - 36.),
                );
                if occupied
                    .iter()
                    .all(|p| (p.x - test.x).abs() > 175. || (p.y - test.y).abs() > 33.)
                {
                    position = test;
                    break;
                }
            }
            occupied.push(position);
        }
        layout.push((anchor, position, label, c, inside));
    }
    for (tag, mut n, mut t, mut color, mut transform, mut bg, mut pick) in &mut symbols {
        n.display = Display::None;
        pick.0 = None;
        if !tactical {
            continue;
        }
        if let Some((anchor, position, label, c, inside)) = layout.get(tag.0) {
            if *inside {
                n.display = Display::Flex;
                transform.rotation = Rot2::IDENTITY;
                if tag.1 == 0 {
                    n.left = px(anchor.x - 4.);
                    n.top = px(anchor.y - 7.);
                    n.width = px(10);
                    n.height = px(15);
                    t.0 = "+".into();
                    color.0 = **c;
                    bg.0 = Color::NONE;
                } else if tag.1 == 1 {
                    pick.0 = pick_ids[tag.0];
                    n.left = px(position.x);
                    n.top = px(position.y);
                    n.width = px(170);
                    n.height = px(32);
                    t.0 = (*label).clone();
                    color.0 = **c;
                    bg.0 = Color::srgb(0.012, 0.035, 0.047);
                } else {
                    let end = *position + Vec2::new(0., 8.);
                    let delta = end - *anchor;
                    let length = delta.length();
                    n.left = px((anchor.x + end.x - length) * 0.5);
                    n.top = px((anchor.y + end.y) * 0.5);
                    n.width = px(length);
                    n.height = px(1);
                    transform.rotation = Rot2::radians(delta.y.atan2(delta.x));
                    t.0.clear();
                    bg.0 = Color::srgba(0.4, 0.7, 0.7, 0.5);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn click(a: &mut App, id: u8) {
        let entries: Vec<_> = a
            .world_mut()
            .query::<(Entity, &Action)>()
            .iter(a.world())
            .map(|(e, t)| (e, t.0))
            .collect();
        for (e, action) in entries {
            *a.world_mut().get_mut::<Interaction>(e).unwrap() = if action == id {
                Interaction::Pressed
            } else {
                Interaction::None
            };
        }
        a.update();
    }
    fn choose(a: &mut App, id: u8) {
        let entries: Vec<_> = a
            .world_mut()
            .query::<(Entity, &super::super::setup_screen::Choice)>()
            .iter(a.world())
            .map(|(e, t)| (e, t.0))
            .collect();
        for (e, choice) in entries {
            *a.world_mut().get_mut::<Interaction>(e).unwrap() = if choice == id {
                Interaction::Pressed
            } else {
                Interaction::None
            };
        }
        a.update();
    }
    #[test]
    fn actual_setup_buttons_start_selected_force_mission_and_difficulty_and_control_radar() {
        let mut a = App::new();
        a.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Image>>()
            .init_resource::<Menu>()
            .init_resource::<crate::warp::Warp>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<AccumulatedMouseMotion>()
            .init_resource::<AccumulatedMouseScroll>()
            .init_resource::<ViewState>()
            .insert_resource(Flight::runway())
            .init_resource::<Combat>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    controls,
                    super::super::setup_screen::controls,
                    crate::combat::controls,
                    draw,
                    super::super::setup_screen::draw,
                )
                    .chain(),
            );
        a.world_mut().spawn(Window::default());
        a.update();
        click(&mut a, 0);
        assert!(a.world().resource::<Flight>().paused);
        choose(&mut a, 22);
        choose(&mut a, 32);
        choose(&mut a, 41);
        choose(&mut a, 51);
        let c = a.world().resource::<Combat>();
        let b = c.bvr.as_ref().unwrap();
        assert_eq!((b.enemies, b.allies), (4, 1));
        assert_eq!(b.difficulty, Difficulty::Hard);
        assert!(b.defense);
        assert_eq!(a.world().resource::<ViewState>().scenario, 8);
        click(&mut a, 5);
        click(&mut a, 6);
        click(&mut a, 8);
        click(&mut a, 9);
        let b = a.world().resource::<Combat>().bvr.as_ref().unwrap();
        assert_eq!(b.radar.range, 3);
        assert_eq!(b.radar.elevation, 5.);
        assert_eq!(b.radar.mode, RadarMode::Stt);
        assert!(a.world().resource::<Menu>().reveal);
        a.world_mut().resource_mut::<ViewState>().map = true;
        a.update();
        let n = a
            .world_mut()
            .query_filtered::<&Node, With<MapImage>>()
            .single(a.world())
            .unwrap();
        assert_eq!(n.display, Display::Flex);
        let contact = a
            .world_mut()
            .query::<(Entity, &PickTarget)>()
            .iter(a.world())
            .find(|(_, p)| p.0 == Some(1))
            .map(|(e, _)| e)
            .unwrap();
        *a.world_mut().get_mut::<Interaction>(contact).unwrap() = Interaction::Pressed;
        a.update();
        assert_eq!(a.world().resource::<Combat>().selected, 1);
        for paused in [false, true] {
            a.world_mut().resource_mut::<Flight>().paused = paused;
            click(&mut a, 0);
            assert!(a.world().resource::<Flight>().paused);
            choose(&mut a, 50);
            assert_eq!(a.world().resource::<Flight>().paused, paused);
        }
    }
}
