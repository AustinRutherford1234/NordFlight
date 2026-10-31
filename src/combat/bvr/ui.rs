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
            (13, "SPAWN F/A-18 / SHIFT+N"),
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
    for i in 0..128 {
        for kind in 0..4 {
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
                GlobalZIndex(if kind==3 {66}else{65}),
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
    mut spawn_buttons: Query<(&Action,&mut Node)>,
    mut menu: ResMut<Menu>,
    mut v: ResMut<ViewState>,
    mut f: ResMut<Flight>,
    mut combat: ResMut<Combat>,
    warp: Res<crate::warp::Warp>,
    time: Res<Time<bevy::time::Real>>,
) {
    combat.map_reveal=menu.reveal;
    for (action,mut node) in &mut spawn_buttons {
        if action.0==13 {node.display=if combat.naval.is_some(){Display::Flex}else{Display::None};}
    }
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
            13 if v.map => {
                if let Some(mut n)=combat.naval.take() {
                    if n.spawn_player(&mut combat,&mut f,&mut v) {v.map=false;}
                    combat.naval=Some(n);
                }
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
        menu.center = Vec2::new(f.position.x as f32, f.position.z as f32 - if combat.bvr.is_some(){50000.}else{0.});
        if std::env::var("NORD_NAVAL_CAPTURE").is_ok_and(|m|m.starts_with("map-")){menu.span=if std::env::var("NORD_NAVAL_CAPTURE").is_ok_and(|m|m.contains("120km")){300000.}else{40000.};}
        menu.last_generation = v.generation;
        menu.dirty = true;
    }
    let axis = Vec2::new(
        (keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight)) as i32 as f32
            - (keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)) as i32 as f32,
        (keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)) as i32 as f32
            - (keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)) as i32 as f32,
    ).normalize_or_zero();
    if axis != Vec2::ZERO {
        let distance = menu.span * 0.4 * time.delta_secs().min(0.1);
        menu.center += axis * distance;
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
            let selected=super::super::tactical::contacts(&combat,&f,menu.reveal).into_iter().filter(|m|!m.friendly).filter_map(|m|m.aircraft.map(|i|(i,to_screen(m.position,&menu,origin,size).distance(cursor)))).filter(|(_,d)|*d<35.).min_by(|a,b|a.1.total_cmp(&b.1));
            if let Some((i, _)) = selected {
                combat.selected = i;
                combat.lock = 0.;
            }
        }
    }
    if keys.just_pressed(KeyCode::Enter) {
        super::cockpit_radar::command(&mut combat,super::cockpit_radar::Command::Lock);
    }
}
pub fn draw(
    time:Res<Time<bevy::time::Real>>,
    mut last_map:Local<(f64,u64,bool)>,
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
            .map_or_else(||format!("SHARED PICTURE / {} CONTACTS",super::super::tactical::contacts(&combat,&f,menu.reveal).len()), |b| {
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
        let readout = format!(
            "TACTICAL / SHARED PICTURE\n{}\nMAP: {}\nSpan {:.0} km / north up\n\nClick contact: designate\nEnter: single-target lock\nWASD / arrows: pan\nRight-drag: pan\nWheel: zoom\nM: return to flight\n\nScenario configuration is\nunder FLIGHT SETUP.",
            radar,
            if menu.reveal {
                "REVEAL ALL / TRAINING"
            } else {
                "SENSOR CONTACTS"
            },
            menu.span / 1000.
        );
        if t.0 != readout {
            t.0 = readout;
        }
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
        last_map.2=false;
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
    let key=v.generation ^ (menu.span.to_bits()as u64).rotate_left(17) ^ (menu.center.x.to_bits()as u64).rotate_left(3) ^ (menu.center.y.to_bits()as u64).rotate_left(29) ^ (combat.selected as u64).rotate_left(41) ^ u64::from(menu.reveal) ^ (size.x.to_bits()as u64).rotate_left(7) ^ (size.y.to_bits()as u64).rotate_left(13);
    if last_map.2 && last_map.1==key && time.elapsed_secs_f64()-last_map.0<0.1{return;}
    *last_map=(time.elapsed_secs_f64(),key,true);
    let mut marks:Vec<(DVec3,String,Color,bool)>=vec![(f.position,"YOU".into(),Color::srgb(0.4,1.,0.85),false)];
    let mut pick_ids=vec![None];
    if let Some(b)=&combat.bvr {marks.push((b.objective,"OBJECTIVE".into(),Color::srgb(0.5,0.8,1.),false));pick_ids.push(None);}
    for mark in super::super::tactical::contacts(&combat,&f,menu.reveal){
        pick_ids.push(if mark.friendly{None}else{mark.aircraft});
        let selected=mark.aircraft==Some(combat.selected)&&!mark.friendly;
        let color=if selected{Color::srgb(1.,0.75,0.25)}else{super::super::tactical::color(&mark)};
        let weapon=mark.category==super::super::tactical::Category::Weapon;
        let label=if weapon {mark.label}else{format!("{}{}\n{:.1} km / {:.0} m",if selected{"* "}else{""},mark.label,f.position.distance(mark.position)/1000.,mark.position.length()-RADIUS)};
        marks.push((mark.position,label,color,weapon));
    }
    let label_width=(size.x-8.).min(250.);
    let mut layout = Vec::new();
    let mut occupied: Vec<(Vec2, f32)> = Vec::new();
    for (p, label, c, weapon) in &marks {
        let anchor = to_screen(*p, &menu, origin, size);
        let inside = anchor.x >= origin.x
            && anchor.y >= origin.y
            && anchor.x < origin.x + size.x
            && anchor.y < origin.y + size.y;
        let rows:f32=label.lines().map(|line|(line.chars().count() as f32*7.5/label_width).ceil().max(1.)).sum();
        let height = rows * 16. + 4.;
        let mut position = anchor + Vec2::new(10., -8.);
        if inside {
            for attempt in 0..200 {
                let row = attempt % 40;
                let column = attempt / 40;
                let horizontal = match column {
                    0 => 10.,
                    1 => 270.,
                    2 => -250.,
                    3 => 530.,
                    _ => -510.,
                };
                let offset = if row % 2 == 0 {
                    row as f32 / 2.
                } else {
                    -((row + 1) as f32 / 2.)
                } * 36.;
                let test = Vec2::new(
                    (anchor.x + horizontal).clamp(origin.x + 4., origin.x + size.x - label_width - 4.),
                    (anchor.y - 8. + offset).clamp(origin.y + 4., (origin.y + size.y - height - 4.).max(origin.y+4.)),
                );
                if occupied
                    .iter()
                    .all(|(p,h)| (p.x - test.x).abs() > label_width+5. || test.y >= p.y + h + 3. || p.y >= test.y + height + 3.)
                {
                    position = test;
                    break;
                }
            }
            occupied.push((position,height));
        }
        layout.push((anchor, position, label, c, inside, height,weapon));
    }
    for (tag, mut n, mut t, mut color, mut transform, mut bg, mut pick) in &mut symbols {
        n.display = Display::None;
        pick.0 = None;
        if !tactical {
            continue;
        }
        if let Some((anchor, position, label, c, inside, height,weapon)) = layout.get(tag.0) {
            if *inside {
                if tag.1==3 && !**weapon {continue;}
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
                    n.width = px(label_width);
                    n.height = px(*height);
                    t.0 = if **weapon {label.lines().next().unwrap_or("").into()}else{(*label).clone()};
                    color.0 = **c;
                    bg.0 = Color::srgb(0.012, 0.035, 0.047);
                } else if tag.1==3 {
                    n.left=px(position.x);n.top=px(position.y+16.);
                    n.width=px(label_width);n.height=px(18.);
                    t.0=label.lines().nth(1).unwrap_or("").into();
                    let rgba=c.to_srgba();
                    color.0=Color::srgba(rgba.red+(1.-rgba.red)*0.12,rgba.green+(1.-rgba.green)*0.12,rgba.blue+(1.-rgba.blue)*0.12,rgba.alpha);
                    bg.0=Color::NONE;
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

#[cfg(test)]mod shared_map_projection_tests{use super::*;
 #[test]fn carrier_raid_at_120km_fits_300km_map(){let f=crate::combat::naval::flight(true);let mut c=Combat::default();crate::combat::naval::start(&mut c,&f,100,1);let mut menu=Menu::default();menu.center=Vec2::new(f.position.x as f32,f.position.z as f32);menu.span=300000.;let origin=Vec2::new(310.,165.);let size=Vec2::new(1105.,700.);for m in crate::combat::tactical::contacts(&c,&f,true){let p=to_screen(m.position,&menu,origin,size);println!("{} {:?}",m.label,p);assert!((origin.x..origin.x+size.x).contains(&p.x)&&(origin.y..origin.y+size.y).contains(&p.y));}}
}

#[cfg(test)] mod keyboard_map_tests {
 use super::*;
 fn scene()->App {
  let mut a=App::new();a.add_plugins(MinimalPlugins)
   .init_resource::<ButtonInput<KeyCode>>().init_resource::<ButtonInput<MouseButton>>()
   .init_resource::<AccumulatedMouseMotion>().init_resource::<AccumulatedMouseScroll>()
   .init_resource::<Menu>().init_resource::<ViewState>().init_resource::<crate::warp::Warp>()
   .insert_resource(Flight::runway()).init_resource::<Combat>().add_systems(Update,controls);
  a.world_mut().spawn(Window::default());a.world_mut().resource_mut::<ViewState>().map=true;
  // Manual wall-clock advancement keeps map panning independent of pause/time warp.
  a.world_mut().insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_millis(20)));
  a.update();a
 }
 #[test] fn both_key_sets_pan_without_a_cursor_and_stop_when_map_closes() {
  let mut a=scene();
  for (key,axis) in [(KeyCode::KeyW,Vec2::NEG_Y),(KeyCode::ArrowUp,Vec2::NEG_Y),(KeyCode::KeyS,Vec2::Y),(KeyCode::ArrowDown,Vec2::Y),(KeyCode::KeyA,Vec2::NEG_X),(KeyCode::ArrowLeft,Vec2::NEG_X),(KeyCode::KeyD,Vec2::X),(KeyCode::ArrowRight,Vec2::X)] {
   a.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();a.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(key);
   let before=a.world().resource::<Menu>().center;a.update();let delta=a.world().resource::<Menu>().center-before;
   assert!(delta.dot(axis)>0.);assert!(delta.perp_dot(axis).abs()<1.);
  }
  let before=a.world().resource::<Menu>().center;a.world_mut().resource_mut::<ViewState>().map=false;a.update();assert_eq!(a.world().resource::<Menu>().center,before);
  a.world_mut().resource_mut::<ViewState>().map=true;a.world_mut().resource_mut::<crate::warp::Warp>().editing=true;a.update();assert_eq!(a.world().resource::<Menu>().center,before);
 }
}
