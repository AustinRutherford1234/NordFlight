//! Screen-space precision controls for the single selected maneuver node.
use super::*;
#[derive(Component)]
pub struct Field(u8);
fn button(p: &mut ChildSpawnerCommands, id: u8, label: &str, width: Val, field: Option<u8>) {
    p.spawn((
        Button,
        Action(id),
        Node {
            width,
            height: px(24),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.06, 0.15, 0.23)),
    ))
    .with_children(|b| {
        let mut text = b.spawn((
            Text::new(label),
            TextFont {
                font_size: 11.,
                ..default()
            },
            TextColor(Color::srgb(0.85, 0.93, 0.97)),
        ));
        if let Some(field) = field {
            text.insert(Field(field));
        }
    });
}
fn row() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        column_gap: px(4),
        ..default()
    }
}
pub fn setup(c: &mut Commands) {
    c.spawn((
        Root,
        Visibility::Hidden,
        Node {
            position_type: PositionType::Absolute,
            left: px(24),
            top: px(214),
            width: px(310),
            padding: UiRect::all(px(10)),
            row_gap: px(4),
            flex_direction: FlexDirection::Column,
            border_radius: BorderRadius::all(px(8)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.015, 0.035, 0.06, 0.97)),
        ZIndex(40),
    ))
    .with_children(|p| {
        p.spawn((
            Readout,
            Text::new(""),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.68, 0.86, 0.94)),
        ));
        p.spawn(row()).with_children(|r| {
            button(r, 0, "+ ADD NODE", percent(49.), None);
            button(r, 1, "REMOVE NODE", percent(49.), None);
        });
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4),
            row_gap: px(4),
            ..default()
        })
        .with_children(|r| {
            for (id, label) in [
                (2, "FOCUS SHIP"),
                (3, "PRIMARY BODY"),
                (4, "FOCUS MOON"),
                (5, "EARTH–MOON"),
            ] {
                button(r, id, label, percent(49.), None);
            }
        });
        button(p, 6, "PLAN LUNAR TRANSFER", percent(100.), None);
        p.spawn((
            Text::new(
                "Click orbit to place · RMB orbit · wheel zoom\nDim → bright: forward in time",
            ),
            TextFont {
                font_size: 11.,
                ..default()
            },
            TextColor(Color::srgb(0.47, 0.65, 0.74)),
        ));
    });
    c.spawn((
        Root,
        Visibility::Hidden,
        Node {
            position_type: PositionType::Absolute,
            left: px(24),
            bottom: px(20),
            width: px(310),
            padding: UiRect::all(px(10)),
            row_gap: px(5),
            flex_direction: FlexDirection::Column,
            border_radius: BorderRadius::all(px(8)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.045, 0.075, 0.98)),
        ZIndex(41),
    ))
    .with_children(|p| {
        p.spawn((
            Field(0),
            Text::new("MANEUVER EDITOR"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(1., 0.75, 0.36)),
        ));
        for (axis, name, color) in [
            (0, "PROGRADE", Color::srgb(0.3, 1., 0.5)),
            (1, "RADIAL", Color::srgb(0.35, 0.8, 1.)),
            (2, "NORMAL", Color::srgb(0.8, 0.6, 1.)),
        ] {
            p.spawn(row()).with_children(|r| {
                r.spawn((
                    Text::new(name),
                    TextFont {
                        font_size: 11.,
                        ..default()
                    },
                    TextColor(color),
                    Node {
                        width: px(78),
                        align_self: AlignSelf::Center,
                        ..default()
                    },
                ));
                button(r, 10 + axis * 2, "−", px(28), None);
                r.spawn((
                    Field(3 + axis),
                    Text::new("—"),
                    TextFont {
                        font_size: 14.,
                        ..default()
                    },
                    TextColor(color),
                    Node {
                        width: px(98),
                        align_self: AlignSelf::Center,
                        ..default()
                    },
                ));
                button(r, 11 + axis * 2, "+", px(28), None);
            });
        }
        p.spawn(row()).with_children(|r| {
            button(r, 20, "", percent(49.), Some(1));
            button(r, 21, "", percent(49.), Some(2));
        });
        p.spawn(row()).with_children(|r| {
            button(r, 30, "", percent(49.), Some(7));
            button(r, 31, "", percent(49.), Some(8));
        });
        p.spawn(row()).with_children(|r| {
            button(r, 32, "ZERO Δv", percent(49.), None);
            button(r, 33, "ALIGN SAS TO NODE", percent(49.), None);
        });
        button(p, 34, "WARP TO MANEUVER · BURN −15s", percent(100.), None);
        p.spawn((
            Field(6),
            Text::new(""),
            TextFont {
                font_size: 11.,
                ..default()
            },
            TextColor(Color::srgb(0.62, 0.77, 0.85)),
        ));
    });
}
pub fn dv_step(p: &Planner) -> f64 {
    [1., 10., 100., 0.1][p.step % 4]
}
fn time_step(p: &Planner) -> f64 {
    [60., 10., 1., 300.][p.time_step % 4]
}
pub fn apply(id: u8, p: &mut Planner, f: &mut Flight) {
    if id == 20 {
        p.step = (p.step + 1) % 4;
        return;
    }
    if id == 21 {
        p.time_step = (p.time_step + 1) % 4;
        return;
    }
    let step = dv_step(p);
    let time = time_step(p);
    let Some(mut n) = p.node else {
        p.message = "Add a node or click the current orbit.".into();
        return;
    };
    match id {
        10..=15 => {
            let axis = ((id - 10) / 2) as usize;
            let sign = if id % 2 == 0 { -1. } else { 1. };
            n.dv[axis] = (n.dv[axis] + sign * step).clamp(-5000., 5000.);
            p.message.clear();
        }
        30 | 31 => {
            let eta = n.eta(f).unwrap_or(0.);
            n.set_eta(f, eta + if id == 30 { -time } else { time });
            p.message.clear();
        }
        32 => {
            n.dv = DVec3::ZERO;
            p.message = "Components reset; node position retained.".into();
        }
        33 => {
            f.sas_mode = 5;
            f.assist = true;
            p.message = "Maneuver SAS on · throttle remains manual.".into();
        }
        _ => return,
    }
    p.node = Some(n);
}
pub fn draw(
    f: Res<Flight>,
    v: Res<ViewState>,
    p: Res<Planner>,
    mut fields: Query<(&Field, &mut Text)>,
    mut buttons: Query<(&Action, &Interaction, &mut BackgroundColor)>,
) {
    if !v.map || f.vehicle != Vehicle::Spacecraft {
        return;
    }
    let mut header =
        "MANEUVER EDITOR\nNo node selected\nAdd a node or click the blue orbit.".to_string();
    let mut status = "Drag the node to move it along the orbit.".to_string();
    if let Some(n) = p.node {
        let eta = n.eta(&f).unwrap_or(0.);
        let dv = n.dv.length();
        let (thrust, isp) = f.space.stage.engine();
        let fuel = f.mass() * (1. - (-dv / (isp * crate::space::G0).max(1.)).exp());
        let burn = if thrust > 0. {
            fuel / (thrust / (isp * crate::space::G0))
        } else {
            0.
        };
        let post = n.states(&f).map(|s| s.2);
        let ap = post
            .as_ref()
            .and_then(|o| o.apoapsis)
            .map(|a| format!("{:.0}", a / 1000.))
            .unwrap_or("ESC".into());
        let pe = post.as_ref().map(|o| o.periapsis / 1000.).unwrap_or(0.);
        header = format!(
            "MANEUVER 01 · Δv {:.1} m/s\nT−{:.0}s · START T−{:.0}s · BURN ≈{:.1}s\nPLANNED AP {} / PE {:.1} km",
            dv,
            eta,
            eta - burn * 0.5,
            burn,
            ap,
            pe
        );
        status = if dv > crate::space::delta_v(&f) {
            "INSUFFICIENT STAGE Δv".into()
        } else if pe < 0. {
            "WARNING · SURFACE INTERSECTION".into()
        } else if pe < 120. && f.space.body == crate::space::Body::Earth {
            "WARNING · ATMOSPHERIC ENTRY".into()
        } else if !p.message.is_empty() {
            p.message.clone()
        } else {
            "P prograde · R radial out · N orbit normal".into()
        };
    }
    for (field, mut text) in &mut fields {
        let next = match field.0 {
            0 => header.clone(),
            1 => format!("Δv STEP {:.1} m/s", dv_step(&p)),
            2 => format!("TIME STEP {:.0}s", time_step(&p)),
            3..=5 => p
                .node
                .map(|n| format!("{:+.1} m/s", n.dv[(field.0 - 3) as usize]))
                .unwrap_or("—".into()),
            6 => status.clone(),
            7 => format!("EARLIER −{:.0}s", time_step(&p)),
            _ => format!("LATER +{:.0}s", time_step(&p)),
        };
        if text.0 != next {
            text.0 = next;
        }
    }
    for (action, i, mut color) in &mut buttons {
        color.0 = if p.node.is_none() && matches!(action.0,10..=15|30..=34) {
            Color::srgb(0.035, 0.065, 0.09)
        } else {
            match i {
                Interaction::Pressed => Color::srgb(0.1, 0.39, 0.34),
                Interaction::Hovered => Color::srgb(0.10, 0.25, 0.34),
                _ => Color::srgb(0.06, 0.15, 0.23),
            }
        };
    }
}
