use crate::physics::{Flight, Vehicle};
use bevy::prelude::*;

#[derive(Component)]
pub struct Gauge(u8);
#[derive(Component)]
pub struct Readout;
#[derive(Component)]
pub struct Warning;
#[derive(Component)]
pub struct FlightButton(u8);
#[derive(Component)]
pub struct Overlay;
pub fn setup(mut c: Commands) {
    for (tag, label) in [(0, "SAS / T"), (1, "MAP"), (2, "PAUSE"), (3, "MODE / F7")] {
        let button = c
            .spawn((
                Button,
                FlightButton(tag),
                crate::Chrome,
                Node {
                    position_type: PositionType::Absolute,
                    top: px(91),
                    right: px(26.0 + (3 - tag) as f32 * 105.0),
                    width: px(97),
                    height: px(32),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.03, 0.07, 0.09, 0.9)),
                BorderColor::all(Color::srgba(0.31, 0.46, 0.53, 0.65)),
                ZIndex(26),
            ))
            .id();
        let label = c
            .spawn((
                Text::new(label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.76, 0.87, 0.90)),
            ))
            .id();
        c.entity(button).add_child(label);
    }
    let panel = c
        .spawn((
            Overlay,
            Node {
                position_type: PositionType::Absolute,
                left: percent(50),
                bottom: px(38),
                margin: UiRect::left(px(-252)),
                width: px(135),
                height: px(184),
                padding: UiRect::all(px(12)),
                flex_direction: FlexDirection::Column,
                row_gap: px(9),
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.038, 0.052, 0.9)),
            BorderColor::all(Color::srgba(0.31, 0.46, 0.53, 0.65)),
            ZIndex(25),
        ))
        .id();
    for (tag, label, color) in [
        (0, "THROTTLE", Color::srgb(0.97, 0.69, 0.22)),
        (1, "FUEL", Color::srgb(0.34, 0.83, 0.60)),
        (2, "AIRFRAME", Color::srgb(0.33, 0.69, 0.95)),
    ] {
        let text = c
            .spawn((
                Text::new(label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.65, 0.76, 0.80)),
            ))
            .id();
        c.entity(panel).add_child(text);
        let track = c
            .spawn((
                Node {
                    width: percent(100),
                    height: px(9),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.09, 0.13, 0.16)),
            ))
            .id();
        let fill = c
            .spawn((
                Node {
                    width: percent(0),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(color),
                Gauge(tag),
            ))
            .id();
        c.entity(track).add_child(fill);
        c.entity(panel).add_child(track);
    }
    c.spawn((
        Text::new(""),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.76, 0.87, 0.90)),
        Node {
            position_type: PositionType::Absolute,
            left: percent(50),
            bottom: px(65),
            margin: UiRect::left(px(120)),
            width: px(164),
            padding: UiRect::all(px(10)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.025, 0.038, 0.052, 0.90)),
        ZIndex(25),
        Readout,
        Overlay,
    ));
    c.spawn((
        Text::new(""),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.65, 0.25)),
        Node {
            position_type: PositionType::Absolute,
            top: px(144),
            left: percent(50),
            margin: UiRect::left(px(-155)),
            width: px(310),
            ..default()
        },
        ZIndex(25),
        Warning,
    ));
}
pub fn buttons(
    mut buttons: Query<(Ref<Interaction>, &FlightButton, &mut BackgroundColor, &mut BorderColor)>,
    mut f: ResMut<Flight>,
    mut view: ResMut<crate::ViewState>,
) {
    for (interaction, tag, _, _) in &mut buttons {
        if interaction.is_changed() && *interaction == Interaction::Pressed {
            if view.left_camera_drag {
                continue;
            }
            match tag.0 {
                0 => f.assist = !f.assist,
                1 => view.toggle_map(&f),
                2 => f.paused = !f.paused,
                _ => {
                    if f.vehicle == Vehicle::Spacecraft {
                        f.sas_mode = (f.sas_mode + 1) % crate::space::SAS_MODES.len() as u8;
                        f.assist = true;
                    }
                }
            };
        }
    }
    for (interaction, tag, mut color, mut border) in &mut buttons {
        let active = tag.0 == 0 && f.assist;
        color.set_if_neq(BackgroundColor(if active {
            if *interaction == Interaction::Hovered {
                Color::srgba(0.08, 0.45, 0.33, 0.98)
            } else {
                Color::srgba(0.04, 0.32, 0.23, 0.98)
            }
        } else {
            match *interaction {
                Interaction::Pressed => Color::srgba(0.07, 0.30, 0.25, 0.95),
                Interaction::Hovered => Color::srgba(0.10, 0.20, 0.24, 0.95),
                Interaction::None => Color::srgba(0.03, 0.07, 0.09, 0.9),
            }
        }));
        border.set_if_neq(BorderColor::all(if active {
            Color::srgb(0.36, 1.0, 0.70)
        } else {
            Color::srgba(0.31, 0.46, 0.53, 0.65)
        }));
    }
}

#[cfg(test)]
mod button_tests {
    use super::*;
    #[test]
    fn sas_highlight_tracks_state_without_hover_or_repeating_a_held_click() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Flight::spacecraft())
            .insert_resource(crate::ViewState::default())
            .add_systems(Update, buttons);
        app.world_mut().resource_mut::<Flight>().assist = false;
        let sas = app.world_mut().spawn((FlightButton(0), Interaction::None,
            BackgroundColor::default(), BorderColor::default())).id();
        let mode = app.world_mut().spawn((FlightButton(3), Interaction::None,
            BackgroundColor::default(), BorderColor::default())).id();
        app.update();
        let off = *app.world().get::<BackgroundColor>(sas).unwrap();
        // State changes through a keyboard/control path without mouse movement.
        app.world_mut().resource_mut::<Flight>().assist = true;
        app.update();
        let on = *app.world().get::<BackgroundColor>(sas).unwrap();
        assert_ne!(on, off);
        *app.world_mut().get_mut::<Interaction>(sas).unwrap() = Interaction::Hovered;
        app.update();
        *app.world_mut().get_mut::<Interaction>(sas).unwrap() = Interaction::None;
        app.update();
        assert_eq!(*app.world().get::<BackgroundColor>(sas).unwrap(), on);
        *app.world_mut().get_mut::<Interaction>(sas).unwrap() = Interaction::Pressed;
        app.update();
        assert!(!app.world().resource::<Flight>().assist);
        app.update();
        assert!(!app.world().resource::<Flight>().assist);
        *app.world_mut().get_mut::<Interaction>(sas).unwrap() = Interaction::None;
        *app.world_mut().get_mut::<Interaction>(mode).unwrap() = Interaction::Pressed;
        app.update();
        assert!(app.world().resource::<Flight>().assist);
        assert_eq!(*app.world().get::<BackgroundColor>(sas).unwrap(), on);
    }
}
pub fn update(
    f: Res<Flight>,
    view: Res<crate::ViewState>,
    time: Res<Time>,
    mut last: Local<f32>,
    mut gauges: Query<(&Gauge, &mut Node)>,
    mut text: Query<&mut Text, With<Readout>>,
    mut warnings: Query<&mut Text, (With<Warning>, Without<Readout>)>,
) {
    if time.elapsed_secs() - *last < 0.1 {
        return;
    }
    *last = time.elapsed_secs();
    let overlay_visible = !view.map && view.index != 3 && !view.simple && !view.following();
    if overlay_visible {
        for (g, mut node) in &mut gauges {
            let width = percent(
                match g.0 {
                    0 => f.throttle * 100.0,
                    1 => {
                        f.fuel
                            / if f.vehicle == Vehicle::Gripen {
                                1500.0
                            } else {
                                f.space.stage.capacity().max(1.)
                            }
                            * 100.0
                    }
                    _ => f.integrity * 100.0,
                }
                .clamp(0.0, 100.0) as f32,
            );
            if node.width != width {
                node.width = width;
            }
        }
        for mut t in &mut text {
            let updated = if f.vehicle == Vehicle::Gripen {
                format!(
                    "FLIGHT COMPUTER\n\nq  {:.1} kPa\nL/D  {:.1}\nAoA  {:+.1}°\nFLOW  {:.0}%\nFBW  {}",
                    0.5 * f.density * f.airspeed.powi(2) / 1000.0,
                    f.aero_coeff.lift / f.aero_coeff.drag.max(0.001),
                    f.alpha.to_degrees(),
                    f.flow.attached * 100.0,
                    if f.aircraft==crate::aircraft::Aircraft::Hornet{crate::aircraft::hornet::fbw::label(&f)}else if f.assist { "ON" } else { "OFF" }
                )
            } else {
                format!(
                    "PROPULSION\n\nTHRUST  {:.0} kN\nTWR  {:.2}\nFUEL  {:.0} kg\nIsp  {:.0} s\nSAS  {}",
                    f.thrust / 1000.0,
                    f.thrust
                        / (f.mass() * f.space.body.mu()
                            / (f.altitude() + f.space.body.radius()).powi(2)),
                    f.fuel,
                    f.space.stage.engine().1,
                    crate::space::sas_label(&f)
                )
            };
            if t.0 != updated {
                t.0 = updated;
            }
        }
    }
    for mut t in &mut warnings {
        let updated = if f.crashed {
            "VEHICLE LOST / R TO RESET".into()
        } else if !f.engine
            && (f.vehicle == Vehicle::Gripen || f.space.stage != crate::space::Stage::Capsule)
        {
            "ENGINE OFF".into()
        } else if f.fuel < 100.0
            && !f.infinite_fuel
            && (f.vehicle == Vehicle::Gripen || f.space.stage != crate::space::Stage::Capsule)
        {
            "LOW FUEL".into()
        } else if f.vehicle == Vehicle::Spacecraft && f.space.rcs_fuel < 20. && !f.infinite_fuel {
            "LOW RCS PROPELLANT".into()
        } else if f.vehicle == Vehicle::Gripen
            && !f.landed
            && f.airspeed > 30.0
            && f.flow.attached < 0.5
        {
            "STALL / REDUCE ANGLE OF ATTACK".into()
        } else if f.vehicle == Vehicle::Gripen && 0.5 * f.density * f.airspeed.powi(2) > 55000.0 {
            "HIGH DYNAMIC PRESSURE".into()
        } else if f.vehicle == Vehicle::Gripen && f.g_load.abs() > 8.5 {
            "HIGH G LOAD".into()
        } else {
            String::new()
        };
        if t.0 != updated {
            t.0 = updated;
        }
    }
}
