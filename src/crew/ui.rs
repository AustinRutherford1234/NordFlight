//! One movable/resizable live portrait and a native crew wardrobe/editor.
use super::*;
use bevy::window::PrimaryWindow;
#[derive(Component)]
pub(super) struct Panel(u8);
#[derive(Component)]
pub(super) struct Action(u16);
#[derive(Component)]
pub(super) struct Status;
#[derive(Component)]
pub(super) struct DragHandle;
#[derive(Component)]
pub(super) struct ResizeHandle;
#[derive(Component)]
pub(super) struct EditorScroll;
fn text(p: &mut ChildSpawnerCommands, s: impl Into<String>, size: f32) {
    p.spawn((
        Text::new(s),
        TextFont {
            font_size: size,
            ..default()
        },
        TextColor(Color::srgb(0.88, 0.93, 0.95)),
    ));
}
fn button(p: &mut ChildSpawnerCommands, id: u16, s: impl Into<String>, selected: bool) {
    p.spawn((
        Button,
        Action(id),
        Node {
            min_height: px(29),
            padding: UiRect::axes(px(9), px(6)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(if selected {
            Color::srgb(0.13, 0.32, 0.34)
        } else {
            Color::srgb(0.035, 0.067, 0.09)
        }),
        BorderColor::all(if selected {
            Color::srgb(0.47, 0.77, 0.72)
        } else {
            Color::srgb(0.15, 0.25, 0.30)
        }),
    ))
    .with_children(|p| text(p, s, 12.));
}
fn row(p: &mut ChildSpawnerCommands, f: impl FnOnce(&mut ChildSpawnerCommands)) {
    p.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: px(5),
        row_gap: px(5),
        ..default()
    })
    .with_children(f);
}
fn heading(p: &mut ChildSpawnerCommands, s: impl Into<String>) {
    p.spawn(Node {
        margin: UiRect::top(px(8)),
        ..default()
    })
    .with_children(|p| text(p, s, 12.));
}
pub fn setup(
    mut c: Commands,
    image: Res<PortraitImage>,
    crew: Res<Crew>,
    cameras: Query<Entity, With<FlightCamera>>,
) {
    let Ok(camera) = cameras.single() else { return };
    c.spawn((
        Panel(0),
        UiTargetCamera(camera),
        GlobalZIndex(90),
        Node {
            position_type: PositionType::Absolute,
            right: px(18),
            top: px(62),
            ..default()
        },
    ))
    .with_children(|p| button(p, 1, "DOG CREW", false));
    c.spawn((
        Panel(1),
        UiTargetCamera(camera),
        GlobalZIndex(90),
        Node {
            position_type: PositionType::Absolute,
            right: px(18),
            bottom: px(18),
            width: px(crew.size.x),
            height: px(crew.size.y),
            flex_direction: FlexDirection::Column,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(8)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::srgb(0.012, 0.027, 0.038)),
        BorderColor::all(Color::srgb(0.25, 0.43, 0.46)),
    ))
    .with_children(|p| {
        p.spawn((
            Button,
            DragHandle,
            Node {
                height: px(28),
                flex_shrink: 0.,
                padding: UiRect::axes(px(10), px(5)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.07, 0.13, 0.16)),
        ))
        .with_children(|p| text(p, "DOG CREW  /  DRAG TO MOVE", 11.));
        p.spawn((
            render::PortraitSurface,
            ImageNode::new(image.0.clone()),
            Node {
                width: percent(100),
                flex_grow: 1.,
                min_height: px(0),
                ..default()
            },
        ));
        p.spawn((
            Status,
            Text::new("OTTO / DACHSHUND"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.85, 0.93, 0.90)),
            Node {
                height: px(24),
                margin: UiRect::left(px(8)),
                ..default()
            },
        ));
        row(p, |p| {
            button(p, 1, "CREW OPTIONS", false);
        });
        p.spawn((
            Button,
            ResizeHandle,
            Node {
                position_type: PositionType::Absolute,
                right: px(0),
                bottom: px(0),
                width: px(22),
                height: px(24),
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.13, 0.23, 0.27)),
        ))
        .with_children(|p| text(p, "↘", 14.));
    });
}
fn editor(c: &mut Commands, camera: Entity, crew: &Crew, image: &PortraitImage) {
    let a = crew.appearances[crew.selected];
    c.spawn((Panel(2),UiTargetCamera(camera),GlobalZIndex(650),Node{position_type:PositionType::Absolute,right:px(18),top:px(18),bottom:px(18),width:px(494),max_width:percent(52),padding:UiRect::all(px(16)),flex_direction:FlexDirection::Column,row_gap:px(7),overflow:Overflow::clip(),border:UiRect::all(px(1)),border_radius:BorderRadius::all(px(10)),..default()},
        BackgroundColor(Color::srgba(0.012,0.025,0.038,0.98)),BorderColor::all(Color::srgb(0.29,0.47,0.49))))
        .with_children(|p|{
            row(p,|p|{text(p,"DOG CREW / FLIGHT ROSTER",18.);button(p,2,"DONE",false);});
            text(p,format!("{}  /  {}  /  {}",NAMES[a.breed],BREEDS[a.breed],TRAITS[a.breed]),13.);
            p.spawn((render::PortraitSurface,ImageNode::new(image.0.clone()),Node{width:px(290),height:px(290),align_self:AlignSelf::Center,flex_shrink:0.,..default()}));
            p.spawn((EditorScroll,ScrollPosition(Vec2::new(0.,crew.editor_scroll)),Node{flex_grow:1.,min_height:px(0),flex_direction:FlexDirection::Column,row_gap:px(7),overflow:Overflow::scroll_y(),..default()})).with_children(|p|{
            heading(p,"CHOOSE YOUR PILOT");
            row(p,|p|{for (i,breed)in BREEDS.iter().enumerate(){button(p,100+i as u16,format!("{:02} {}",i+1,breed),i==crew.selected);}});
            heading(p,"WARDROBE / CHANGES ARE SAVED");
            row(p,|p|{button(p,200,format!("COAT: {}",["ORIGINAL","LIGHT","DARK","CHARCOAL","CREAM","GOLDEN","SILVER"][a.coat]),false);button(p,201,format!("MARKING: {}",["BREED","BLAZE","EYE PATCH"][a.marking]),false);button(p,202,format!("SUIT: {}",["RESCUE ORANGE","SAGE","NAVY"][a.suit]),false);});
            row(p,|p|{for (i,label)in ["OPEN HELMET","HEADSET","OXYGEN MASK"].into_iter().enumerate(){button(p,210+i as u16,label,a.equipment==i);}});
            row(p,|p|{button(p,220,if crew.chatter{"CHATTER ON"}else{"CHATTER OFF"},crew.chatter);button(p,221,"RESET PORTRAIT POSITION",false);});
            heading(p,"PREVIEW / EMOTIONS + G LOADS (SCROLL)");
            row(p,|p|{for (id,preview)in [(300,Preview::Live),(308,Preview::Lost)]{button(p,id,preview.label(),crew.preview==preview);}});
            row(p,|p|{for (i,e)in Emotion::ALL.into_iter().enumerate(){button(p,400+i as u16,e.label(),crew.preview==Preview::Emotion(e));}});
            if let Preview::Emotion(e)=crew.preview {text(p,e.description(),12.);}
            heading(p,"G-LOC / PASS OUT & RECOVERY");
            row(p,|p|{for (i,phase)in conscious::Preview::ALL.into_iter().enumerate(){button(p,600+i as u16,phase.label(),crew.preview==Preview::Faint(phase));}});
            if let Preview::Faint(phase)=crew.preview {text(p,phase.description(),12.);}
            heading(p,"ACCELERATION / CONTINUOUS IN LIVE FLIGHT");
            row(p,|p|{for (i,load)in loads::Preset::ALL.into_iter().enumerate(){button(p,500+i as u16,load.label(),crew.preview==Preview::Load(load));}});
            if let Preview::Load(load)=crew.preview {text(p,load.description(),12.);}
            text(p,"Directions name the aircraft's acceleration. The pilot leans the opposite way. Scroll for all previews.",12.);
            text(p,"F-22 first fitting • Otto is the default pilot. All ten looks share the animated rig; the Dachshund is the first detail pass.",12.);
            text(p,"Portraits follow the viewed aircraft and hide in missile view. Drag the portrait's top bar to move it; drag its lower-right corner to resize.",12.);
            });
        });
}
fn apply_action(crew: &mut Crew, id: u16) {
    match id {
        1 => {
            crew.editor = !crew.editor;
            crew.preview = Preview::Live;
        }
        2 => {
            crew.editor = false;
            crew.preview = Preview::Live;
        }
        100..=109 => crew.selected = (id - 100) as usize,
        200..=202 => {
            let selected = crew.selected;
            let a = &mut crew.appearances[selected];
            match id {
                200 => a.coat = (a.coat + 1) % 7,
                201 => a.marking = (a.marking + 1) % 3,
                _ => a.suit = (a.suit + 1) % 3,
            }
        }
        210..=212 => {
            let selected = crew.selected;
            crew.appearances[selected].equipment = (id - 210) as usize;
        }
        220 => crew.chatter = !crew.chatter,
        221 => {
            crew.panel = Vec2::splat(-1.);
            crew.size = Vec2::new(244., 284.);
        }
        300..=308 => {
            crew.preview = [
                Preview::Live,
                Preview::Calm,
                Preview::Positive,
                Preview::Negative,
                Preview::Sideways,
                Preview::Explosion,
                Preview::Warning,
                Preview::Happy,
                Preview::Lost,
            ][(id - 300) as usize]
        }
        500..=514=>crew.preview=Preview::Load(loads::Preset::ALL[(id-500)as usize]),
        400..=419 => crew.preview = Preview::Emotion(Emotion::ALL[(id - 400) as usize]),
        _ => {}
    }
}

pub fn controls(
    mut crew: ResMut<Crew>,
    menu: Res<crate::start_screen::Menu>,
    v: Res<ViewState>,
    f: Res<Flight>,
    time: Res<Time>,
    window: Query<&Window, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    actions: Query<(&Interaction, &Action), Changed<Interaction>>,
    handles: Query<&Interaction, With<DragHandle>>,
    resize: Query<&Interaction, With<ResizeHandle>>,
    mut scroll: ResMut<bevy::input::mouse::AccumulatedMouseScroll>,
    mut editors: Query<&mut ScrollPosition, With<EditorScroll>>,
) {
    for (interaction, action) in &actions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let id = action.0;
        apply_action(&mut crew, id);
        crew.dirty = true;
        crew.save();
    }
    if crew.editor && scroll.delta.y != 0. {
        for mut pos in &mut editors {
            pos.y = (pos.y - scroll.delta.y * 26.).max(0.);
            crew.editor_scroll=pos.y;
        }
        scroll.delta = Vec2::ZERO;
    }
    let Ok(window) = window.single() else { return };
    let cursor = window.cursor_position();
    let can_drag = !menu.open && !crew.editor && v.missile_follow.is_none() && supported(&f);
    if mouse.just_pressed(MouseButton::Left) && can_drag {
        crew.dragging = handles.iter().any(|i| *i == Interaction::Pressed);
        crew.resizing = resize.iter().any(|i| *i == Interaction::Pressed);
        if (crew.dragging || crew.resizing) && crew.panel.x < 0. {
            crew.panel = Vec2::new(
                window.width() - crew.size.x - 18.,
                window.height() - crew.size.y - 18.,
            );
        }
    }
    if mouse.pressed(MouseButton::Left) {
        if let (Some(now), Some(previous)) = (cursor, crew.pointer) {
            let delta = now - previous;
            if crew.dragging {
                crew.panel += delta;
            }
            if crew.resizing {
                crew.size = (crew.size + delta).clamp(Vec2::new(190., 220.), Vec2::new(480., 520.));
            }
        }
    } else if crew.dragging || crew.resizing {
        crew.dragging = false;
        crew.resizing = false;
        crew.save();
    }
    if crew.panel.x >= 0. {
        crew.panel = crew.panel.clamp(
            Vec2::ZERO,
            Vec2::new(
                (window.width() - crew.size.x).max(0.),
                (window.height() - crew.size.y).max(0.),
            ),
        );
    }
    crew.pointer = cursor;
    let _ = time;
}
pub fn draw(
    mut c: Commands,
    mut crew: ResMut<Crew>,
    image: Res<PortraitImage>,
    menu: Res<crate::start_screen::Menu>,
    v: Res<ViewState>,
    f: Res<Flight>,
    combat: Res<Combat>,
    states: Res<Reactions>,
    tracks: Option<Res<Performances>>,
    cameras: Query<Entity, With<FlightCamera>>,
    menus: Query<Entity, With<crate::start_screen::MenuCamera>>,
    mut panels: Query<(Entity, &Panel, &mut Node, &mut UiTargetCamera)>,
    mut status: Query<&mut Text, With<Status>>,
) {
    let camera = if menu.open {
        menus.iter().next()
    } else {
        cameras.iter().next()
    };
    let Some(camera) = camera else { return };
    let watched = owner(&v, &crew);
    let available = flight_for(watched, &f, &combat).is_some_and(supported);
    for (e, panel, mut node, mut target) in &mut panels {
        target.set_if_neq(UiTargetCamera(camera));
        let mut next = (*node).clone();
        let show = match panel.0 {
            0 => !crew.editor,
            1 => !menu.open && !crew.editor && available && v.missile_follow.is_none(),
            _ => crew.editor,
        };
        next.display = if show { Display::Flex } else { Display::None };
        if panel.0 == 1 {
            next.width = px(crew.size.x);
            next.height = px(crew.size.y);
            if crew.panel.x >= 0. {
                next.left = px(crew.panel.x);
                next.top = px(crew.panel.y);
                next.right = Val::Auto;
                next.bottom = Val::Auto;
            } else {
                next.left = Val::Auto;
                next.top = Val::Auto;
                next.right = px(18);
                next.bottom = px(18);
            }
        }
        node.set_if_neq(next);
        if panel.0 == 2 && crew.dirty {
            c.entity(e).despawn();
        }
    }
    if crew.dirty {
        if crew.editor {
            editor(&mut c, camera, &crew, &image);
        }
        crew.dirty = false;
    }
    let a = appearance(&crew, watched);
    let face = states.states[watched].face;
    let conscious=if preview_active(&crew)&&matches!(crew.preview,Preview::Faint(_)){tracks.as_ref().map_or(conscious::State::default(),|t|t.preview_state)}else{states.states[watched].conscious};
    let mood = if face.lost {
        "CREW LOST"
    } else if let Some(label)=conscious.label() {
        label
    } else if face.surprise > 0.5 {
        "STARTLED"
    } else if let Some(label)=states.states[watched].load.label() {
        label
    } else if face.strain > 0.4 {
        "STRAINING"
    } else if face.negative > 0.3 {
        "NEGATIVE G"
    } else if face.worry > 0.4 {
        "ALERT"
    } else if face.joy > 0.4 {
        "NICE LANDING"
    } else {
        states.states[watched].feeling.label()
    };
    for mut label in &mut status {
        let value = format!("{} / {}  •  {}", NAMES[a.breed], BREEDS[a.breed], mood);
        if label.0 != value {
            label.0 = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_ten_breeds_and_three_equipment_choices_are_selectable() {
        let mut crew = Crew::default();
        for breed in 0..10 {
            apply_action(&mut crew, 100 + breed as u16);
            assert_eq!(crew.selected, breed);
            for equipment in 0..3 {
                apply_action(&mut crew, 210 + equipment as u16);
                assert_eq!(crew.appearances[breed].equipment, equipment);
            }
        }
        apply_action(&mut crew, 107);
        apply_action(&mut crew, 200);
        apply_action(&mut crew, 201);
        apply_action(&mut crew, 202);
        assert_eq!(crew.appearances[7].coat, 1);
        assert_eq!(crew.appearances[7].marking, 1);
        assert_eq!(crew.appearances[7].suit, 1);
        apply_action(&mut crew, 1);
        assert!(crew.editor);
        apply_action(&mut crew, 303);
        assert_eq!(crew.preview, Preview::Negative);
        apply_action(&mut crew, 2);
        assert!(!crew.editor);
        assert_eq!(crew.preview, Preview::Live);
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    #[derive(Resource, Default)]
    struct Changes(usize);
    fn count(q: Query<(), Changed<Node>>, mut changes: ResMut<Changes>) {
        changes.0 += q.iter().count();
    }
    #[test]
    fn unchanged_portrait_layout_does_not_invalidate_ui_every_frame() {
        let mut app = App::new();
        let mut f = Flight::runway();
        f.aircraft = Aircraft::F22;
        app.insert_resource(f)
            .init_resource::<Crew>()
            .init_resource::<ViewState>()
            .init_resource::<Combat>()
            .init_resource::<crate::start_screen::Menu>()
            .init_resource::<Reactions>()
            .init_resource::<Changes>()
            .insert_resource(PortraitImage(Handle::default()))
            .add_systems(Update, (draw, count).chain());
        let camera = app.world_mut().spawn(FlightCamera).id();
        app.world_mut()
            .spawn((Panel(1), Node::default(), UiTargetCamera(camera)));
        app.update();
        let initial = app.world().resource::<Changes>().0;
        assert_eq!(initial, 1);
        app.update();
        app.update();
        assert_eq!(app.world().resource::<Changes>().0, initial);
        app.world_mut().resource_mut::<Crew>().size.x += 30.;
        app.update();
        assert_eq!(app.world().resource::<Changes>().0, initial + 1);
    }
    #[test]
    fn actual_combat_panels_dock_clear_of_portrait_without_per_frame_layout_changes() {
        let mut app = App::new();
        let mut f = crate::combat::flight();
        crate::aircraft::select(&mut f, Aircraft::F22);
        let mut combat = Combat::default();
        combat.start_air(&f, 1, 6);
        app.insert_resource(f)
            .insert_resource(combat)
            .init_resource::<Crew>()
            .init_resource::<ViewState>()
            .init_resource::<crate::start_screen::Menu>()
            .init_resource::<Changes>()
            .add_systems(Update, (avoid_portrait, count).chain());
        app.world_mut().spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(1440, 900),
                ..default()
            },
            PrimaryWindow,
        ));
        let weapon = app
            .world_mut()
            .spawn((
                crate::combat::CombatPanel,
                Node {
                    right: px(28),
                    top: px(173),
                    width: px(284),
                    ..default()
                },
            ))
            .id();
        let defense = app
            .world_mut()
            .spawn((
                crate::combat::dogfight::DefensePanel,
                Node {
                    right: px(28),
                    bottom: px(174),
                    width: px(284),
                    ..default()
                },
            ))
            .id();
        app.world_mut().spawn((crate::Hud(7), Node::default()));
        app.update();
        assert_eq!(
            app.world().get::<Node>(weapon).unwrap().top,
            px(layout::COMBAT_TOP)
        );
        assert_eq!(app.world().get::<Node>(weapon).unwrap().right, px(340));
        assert_eq!(
            app.world().get::<Node>(defense).unwrap().top,
            px(layout::COMBAT_TOP + 144.)
        );
        assert_eq!(app.world().get::<Node>(defense).unwrap().right, px(340));
        let changed = app.world().resource::<Changes>().0;
        app.update();
        assert_eq!(app.world().resource::<Changes>().0, changed);
        app.world_mut().resource_mut::<Crew>().size = Vec2::new(480., 520.);
        app.update();
        assert_eq!(app.world().get::<Node>(weapon).unwrap().right, px(510));
        assert_eq!(app.world().get::<Node>(defense).unwrap().right, px(510));
    }
}

/// Dock all fixed combat panels beside the portrait, keeping weather in its
/// own upper-right slot. Stable nodes do not trigger another UI layout pass.
pub(super) fn avoid_portrait(
    crew: Res<Crew>,
    menu: Res<crate::start_screen::Menu>,
    view: Res<ViewState>,
    flight: Res<Flight>,
    combat: Res<Combat>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut hud: Query<
        (&crate::Hud, &ComputedNode, &mut Node),
        (
            Without<crate::combat::CombatPanel>,
            Without<crate::combat::dogfight::DefensePanel>,
        ),
    >,
    mut weapons: Query<
        (&ComputedNode, &mut Node),
        (
            With<crate::combat::CombatPanel>,
            Without<crate::Hud>,
            Without<crate::combat::dogfight::DefensePanel>,
        ),
    >,
    mut defense: Query<
        &mut Node,
        (
            With<crate::combat::dogfight::DefensePanel>,
            Without<crate::Hud>,
            Without<crate::combat::CombatPanel>,
        ),
    >,
) {
    let Ok(window) = windows.single() else { return };
    let size = Vec2::new(window.width(), window.height());
    let portrait = if menu.open {
        None
    } else {
        layout::portrait_bounds(Some(&crew), &flight, &view, &combat, size)
    };
    let dock = portrait.filter(|_| combat.active && !view.map && !view.simple && view.index != 3);
    let right = dock.map(|p| layout::combat_right(size, p));
    for (tag, _, mut node) in &mut hud {
        if tag.0 != 7 {
            continue;
        }
        let mut next = (*node).clone();
        next.top = if portrait.is_some() {
            px(140)
        } else {
            Val::Auto
        };
        next.bottom = if portrait.is_some() {
            Val::Auto
        } else {
            px(40)
        };
        // A fixed maximum keeps long weather values inside their own column.
        next.max_width = if portrait.is_some() {
            px(292)
        } else {
            Val::Auto
        };
        next.right = px(layout::weather_right(size, right));
        node.set_if_neq(next);
    }
    let mut weapons_height = 0.;
    for (computed, mut node) in &mut weapons {
        if combat.bvr.is_none() {
            weapons_height = (computed.size().y * computed.inverse_scale_factor()).max(132.);
        }
        let mut next = (*node).clone();
        next.right = px(right.unwrap_or(28.));
        next.top = px(if right.is_some() {
            layout::COMBAT_TOP
        } else {
            173.
        });
        node.set_if_neq(next);
    }
    for mut node in &mut defense {
        let mut next = (*node).clone();
        next.right = px(right.unwrap_or(28.));
        next.top = right.map_or(Val::Auto, |_| {
            px(layout::COMBAT_TOP
                + if weapons_height > 0. {
                    weapons_height + 12.
                } else {
                    0.
                })
        });
        next.bottom = if right.is_some() { Val::Auto } else { px(174) };
        node.set_if_neq(next);
    }
}
