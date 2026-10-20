//! Scenario configuration is separate from live radar operation.
use super::*;
#[derive(Component)]
pub struct Root;
#[derive(Component)]
pub struct Choice(pub u8);
#[derive(Component)]
pub struct Briefing;

fn choice(p: &mut ChildSpawnerCommands, id: u8, label: &str, width: Val, height: f32) {
    p.spawn((
        Button,
        Choice(id),
        Node {
            width,
            height: px(height),
            padding: UiRect::all(px(15)),
            align_items: AlignItems::Center,
            border: UiRect::all(px(2)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.035, 0.075, 0.105)),
        BorderColor::all(Color::srgb(0.13, 0.24, 0.30)),
    ))
    .with_children(|p| {
        p.spawn((
            Text::new(label),
            TextFont {
                font_size: 18.,
                ..default()
            },
            TextColor(Color::srgb(0.78, 0.89, 0.93)),
        ));
    });
}
fn heading(p: &mut ChildSpawnerCommands, text: &str) {
    p.spawn((
        Text::new(text),
        TextFont {
            font_size: 14.,
            ..default()
        },
        TextColor(Color::srgb(0.44, 0.74, 0.80)),
    ));
}
pub fn spawn(c: &mut Commands) {
    c.spawn((
        Root,
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.008, 0.019, 0.029)),
        GlobalZIndex(95),
    ))
    .with_children(|p| {
        p.spawn(Node {
            width: px(1100),
            max_width: percent(95),
            flex_direction: FlexDirection::Column,
            row_gap: px(14),
            padding: UiRect::all(px(24)),
            ..default()
        })
        .with_children(|p| {
            p.spawn((
                Text::new("BVR / MISSION SETUP"),
                TextFont {
                    font_size: 32.,
                    ..default()
                },
                TextColor(Color::srgb(0.48, 1., 0.85)),
            ));
            p.spawn((
                Text::new("Flight is paused. Choose your flight, difficulty and objective."),
                TextFont {
                    font_size: 16.,
                    ..default()
                },
                TextColor(Color::srgb(0.65, 0.78, 0.84)),
            ));
            heading(p, "01   CHOOSE YOUR FLIGHT");
            p.spawn(Node {
                column_gap: px(12),
                ..default()
            })
            .with_children(|p| {
                for (id, label) in [
                    (20, "SOLO INTERCEPT\n1 vs 2\nYou alone"),
                    (21, "PAIRED FLIGHT\n2 vs 2\nYou + one wingman"),
                    (22, "OUTNUMBERED\n2 vs 4\nYou + one wingman"),
                    (23, "FLIGHT ENGAGEMENT\n4 vs 8\nYou + three allies"),
                ] {
                    choice(p, id, label, percent(25), 115.);
                }
            });
            heading(p, "02   DIFFICULTY");
            p.spawn(Node {
                column_gap: px(12),
                ..default()
            })
            .with_children(|p| {
                for (id, label) in [
                    (30, "TRAINING\nSlower reactions, cautious launches"),
                    (31, "BALANCED\nStandard tactical challenge"),
                    (32, "HARD\nFast reactions, coordinated threats"),
                ] {
                    choice(p, id, label, percent(33.333), 78.);
                }
            });
            heading(p, "03   MISSION");
            p.spawn(Node {
                column_gap: px(12),
                ..default()
            })
            .with_children(|p| {
                choice(
                    p,
                    40,
                    "ELIMINATION\nDefeat the opposing flight",
                    percent(50),
                    78.,
                );
                choice(
                    p,
                    41,
                    "INTERCEPT / DEFEND\nKeep attackers outside the home sector",
                    percent(50),
                    78.,
                );
            });
            p.spawn((
                Briefing,
                Text::new(""),
                TextFont {
                    font_size: 16.,
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.88, 0.91)),
                Node {
                    padding: UiRect::all(px(16)),
                    min_height: px(95),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.02, 0.045, 0.06)),
            ));
            p.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                column_gap: px(18),
                ..default()
            })
            .with_children(|p| {
                choice(p, 50, "CANCEL / RETURN TO FLIGHT", percent(40), 52.);
                choice(p, 51, "START MISSION", percent(40), 52.);
            });
        });
    });
}
pub fn cancel(menu: &mut Menu, f: &mut Flight) {
    if let Some(paused) = menu.saved_pause.take() {
        f.paused = paused;
    }
    menu.open = false;
}
pub fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut menu: ResMut<Menu>,
    mut f: ResMut<Flight>,
    mut v: ResMut<ViewState>,
    buttons: Query<(&Interaction, &Choice), Changed<Interaction>>,
) {
    if !menu.open {
        return;
    }
    let mut start = false;
    let mut close = keys.just_pressed(KeyCode::Escape);
    for (i, choice) in &buttons {
        if *i != Interaction::Pressed {
            continue;
        }
        match choice.0 {
            20..=23 => menu.formation = (choice.0 - 20) as usize,
            30..=32 => {
                menu.difficulty = match choice.0 {
                    30 => Difficulty::Training,
                    31 => Difficulty::Balanced,
                    _ => Difficulty::Hard,
                }
            }
            40..=41 => menu.defense = choice.0 == 41,
            50 => close = true,
            51 => start = true,
            _ => {}
        }
    }
    if start {
        crate::reset(&mut f, 8, &mut v);
        menu.saved_pause = None;
        menu.open = false;
        menu.dirty = true;
    } else if close {
        cancel(&mut menu, &mut f);
    }
}
pub fn draw(
    menu: Res<Menu>,
    mut root: Query<&mut Node, With<Root>>,
    mut cards: Query<(
        &Choice,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut text: Query<&mut Text, With<Briefing>>,
) {
    for mut n in &mut root {
        n.display = if menu.open {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (choice, interaction, mut bg, mut border) in &mut cards {
        let selected = match choice.0 {
            20..=23 => menu.formation == (choice.0 - 20) as usize,
            30 => menu.difficulty == Difficulty::Training,
            31 => menu.difficulty == Difficulty::Balanced,
            32 => menu.difficulty == Difficulty::Hard,
            40 => !menu.defense,
            41 => menu.defense,
            51 => true,
            _ => false,
        };
        bg.0 = if selected {
            Color::srgb(0.055, 0.20, 0.22)
        } else if *interaction == Interaction::Hovered {
            Color::srgb(0.07, 0.13, 0.17)
        } else {
            Color::srgb(0.035, 0.075, 0.105)
        };
        *border = BorderColor::all(if selected {
            Color::srgb(0.32, 0.95, 0.78)
        } else {
            Color::srgb(0.13, 0.24, 0.30)
        });
    }
    for mut t in &mut text {
        let (e, a) = FORMATIONS[menu.formation];
        t.0 = format!(
            "BRIEFING   {} friendly vs {} hostile  /  {}  /  {}\n125 km separation · 11 km starting altitude · mountain-and-valley theater\nYour load: 4 LR-AAM + 2 IR missiles.  M: tactical map.  C: cockpit.\n{}",
            a + 1,
            e,
            menu.difficulty.label(),
            if menu.defense { "DEFEND" } else { "ELIMINATE" },
            if menu.defense {
                "Intercept before an attacker reaches the 18 km home-sector boundary."
            } else {
                "Destroy all hostiles. Support your missiles until their seekers go active."
            }
        );
    }
}
