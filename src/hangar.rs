//! Aircraft selection is a paused, explicit scenario reset, never an in-flight
//! replacement of mass or geometry on a running integrator.
use crate::{ViewState, aircraft::Aircraft, physics::*};
use bevy::prelude::*;
#[derive(Resource, Default)]
pub struct Hangar {
    pub open: bool,
    pub block_input: bool,
    selected: Aircraft,
    was_paused: bool,
    external:bool,
}
#[derive(Component)]
pub(crate) struct Panel;
#[derive(Component)]
pub(crate) struct Toggle;
#[derive(Component)]
pub(crate) struct Choice(Aircraft);
#[derive(Component)]
pub(crate) struct Start;
#[derive(Component)]
pub(crate) struct Cancel;
#[derive(Component)]
pub(crate) struct Readout;
#[derive(Component)] pub(crate) struct Loadout;
fn button(c: &mut ChildSpawnerCommands, label: &str) -> Entity {
    c.spawn((
        Button,
        Node {
            padding: UiRect::all(px(14)),
            margin: UiRect::all(px(5)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.05, 0.13, 0.17)),
    ))
    .with_children(|p| {
        p.spawn((
            Text::new(label),
            TextFont {
                font_size: 17.,
                ..default()
            },
            TextColor(Color::srgb(0.75, 0.94, 0.96)),
        ));
    })
    .id()
}
pub fn setup(mut c: Commands, f: Res<Flight>, mut h: ResMut<Hangar>) {
    h.selected = f.aircraft;
    if std::env::var("NORD_FLEET_CAPTURE").as_deref() == Ok("hangar") {
        h.open = true;
        h.was_paused = f.paused;
    }
    c.spawn((
        Button,
        Toggle,
        Node {
            position_type: PositionType::Absolute,
            left: px(505),
            top: px(20),
            padding: UiRect::all(px(10)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.04, 0.12, 0.16)),
        ZIndex(55),
    ))
    .with_children(|p| {
        p.spawn((
            Text::new("F6 / AIRCRAFT HANGAR"),
            TextFont {
                font_size: 15.,
                ..default()
            },
        ));
    });
    c.spawn((Panel,Node{display:Display::None,position_type:PositionType::Absolute,left:percent(8),top:percent(14),width:percent(84),flex_direction:FlexDirection::Column,padding:UiRect::all(px(26)),border_radius:BorderRadius::all(px(12)),..default()},BackgroundColor(Color::srgba(0.018,0.035,0.055,0.98)),ZIndex(200))).with_children(|p|{
  p.spawn((Text::new("NORD / AIRCRAFT HANGAR"),TextFont{font_size:29.,..default()},TextColor(Color::srgb(0.75,0.95,1.))));
  p.spawn((Text::new("Select your aircraft. AI remains in Gripens. Starting resets the atmospheric scenario."),TextFont{font_size:16.,..default()}));
  p.spawn(Node{flex_direction:FlexDirection::Row,flex_wrap:FlexWrap::Wrap,margin:UiRect::vertical(px(20)),..default()}).with_children(|p|{for a in Aircraft::ALL{let e=button(p,a.name());p.commands().entity(e).insert(Choice(a));}});
  p.spawn((Readout,Text::new(""),TextFont{font_size:18.,..default()},TextColor(Color::srgb(0.65,0.9,0.9)),Node{min_height:px(150),..default()}));
  let e=button(p,"F-22 LOADOUT: TOGGLE INTERNAL / TWO EXTERNAL MISSILES");p.commands().entity(e).insert(Loadout);
  p.spawn(Node{flex_direction:FlexDirection::Row,..default()}).with_children(|p|{let e=button(p,"START / RESET SCENARIO");p.commands().entity(e).insert(Start);let e=button(p,"CANCEL / ESC");p.commands().entity(e).insert(Cancel);});
 });
}
pub fn controls(
    mut bvr: Option<ResMut<crate::combat::bvr::Menu>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut h: ResMut<Hangar>,
    mut f: ResMut<Flight>,
    mut v: ResMut<ViewState>,
    buttons: Query<
        (
            &Interaction,
            Option<&Toggle>,
            Option<&Choice>,
            Option<&Start>,
            Option<&Cancel>,
        ),
        Changed<Interaction>,
    >,
    mut panels: Query<&mut Node, With<Panel>>,
    mut text: Query<&mut Text, With<Readout>>,
    mut combat: ResMut<crate::combat::Combat>,
    mut loadout:Query<(Ref<Interaction>,&mut Visibility),With<Loadout>>,
    mut choices: Query<(&Choice, &mut BackgroundColor)>,
) {
    h.block_input = h.open;
    for (i,mut visibility) in &mut loadout {
        visibility.set_if_neq(if h.selected==Aircraft::F22{Visibility::Inherited}else{Visibility::Hidden});
        if h.open && h.selected==Aircraft::F22 && i.is_changed()&&*i==Interaction::Pressed{h.external=!h.external;}
    }
    let mut toggle = keys.just_pressed(KeyCode::F6)
        && !(keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight));
    let mut cancel = h.open && keys.just_pressed(KeyCode::Escape);
    let mut start = false;
    for (i, t, a, s, c) in &buttons {
        if *i != Interaction::Pressed {
            continue;
        }
        toggle |= t.is_some();
        cancel |= c.is_some();
        start |= s.is_some();
        if let Some(a) = a {
            h.selected = a.0;
        }
    }
    if toggle {
        h.block_input = true;
        if h.open {
            cancel = true;
        } else {
            h.open = true;
            h.selected = f.aircraft;
            h.external=f.systems.external_stores;
            h.was_paused = f.paused;
            if let Some(menu) = bvr.as_mut() {
                if menu.open {
                    h.was_paused = menu.saved_pause.take().unwrap_or(f.paused);
                    menu.open = false;
                }
            }
            f.paused = true;
            combat.stop_gun();
        }
    }
    if cancel && h.open {
        h.open = false;
        f.paused = h.was_paused;
    }
    if start && h.open {
        let a = h.selected;
        let scenario = if f.vehicle == Vehicle::Spacecraft {
            1
        } else {
            v.scenario
        };
        f.aircraft = a;
        crate::reset(&mut f, scenario, &mut v);
        if a==Aircraft::F22{f.systems.external_stores=h.external;}
        f.paused = h.was_paused;
        h.open = false;
    }
    if h.open {
        let s = h.selected.spec();
        let detail = match h.selected {
            Aircraft::Su27 => {
                "Twin AL-31-style engines / conventional tail / T disables FBW for post-stall flight"
            }
            Aircraft::F35B => {
                "Single engine / Y converts to STOVL below 150 m/s / manage throttle, fuel and landing weight"
            }
            Aircraft::F22 => {
                "Twin engines / force-based pitch thrust vectoring / internal weapon stations"
            }
            Aircraft::Hornet => {
                "Twin engines / carrier-style high-lift wing and stabilators / external stores"
            }
            Aircraft::Gripen => {
                "Original canard-delta aircraft / existing flight model and cockpit"
            }
        };
        for mut t in &mut text {
            t.0 = format!(
                "{}\n{}\nSPAN {:.1} m  /  LENGTH {:.1} m  /  WING {:.1} m²\n{}  /  {} ENGINES  /  {:.0} kN WET\nT: FBW ON/OFF   •   ALT+J: JETTISON STORES",
                h.selected.name(),
                detail,
                s.span,
                s.length,
                s.area,
                if h.selected == Aircraft::F35B {
                    "STOVL"
                } else {
                    "CONVENTIONAL"
                },
                s.engines,
                s.wet / 1000.
            );
            if h.selected==Aircraft::F22{t.0.push_str(if h.external{"\nLOADOUT: 2 IR + 4 INTERNAL RADAR / 2 EXTERNAL RADAR"}else{"\nLOADOUT: CLEAN / ALL 8 MISSILES INTERNAL"});}
        }
    }
    for mut p in &mut panels {
        p.display = if h.open { Display::Flex } else { Display::None };
    }
    for (Choice(a), mut bg) in &mut choices {
        bg.set_if_neq(BackgroundColor(if *a == h.selected {
            Color::srgb(0.13, 0.34, 0.37)
        } else {
            Color::srgb(0.05, 0.13, 0.17)
        }));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hangar_cancel_restores_pause_and_start_resets_to_selected_aircraft() {
        let mut app = App::new();
        app.init_resource::<Hangar>()
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(Flight::runway())
            .insert_resource(ViewState::default())
            .insert_resource(crate::combat::Combat::default())
            .add_systems(Update, controls);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F6);
        app.update();
        assert!(app.world().resource::<Hangar>().open);
        assert!(app.world().resource::<Flight>().paused);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        assert!(!app.world().resource::<Flight>().paused);
        assert!(app.world().resource::<Hangar>().block_input);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.update();
        assert!(!app.world().resource::<Hangar>().block_input);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F6);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .spawn((Choice(Aircraft::F22), Interaction::Pressed));
        app.world_mut().spawn((Start, Interaction::Pressed));
        app.update();
        let f = app.world().resource::<Flight>();
        assert_eq!(f.aircraft, Aircraft::F22);
        assert_eq!(f.damage.aircraft, Aircraft::F22);
        assert!(!f.paused);
        assert!(!app.world().resource::<Hangar>().open);
    }
}
pub fn systems(
    keys: Res<ButtonInput<KeyCode>>,
    h: Res<Hangar>,
    mut f: ResMut<Flight>,
    v: Res<ViewState>,
    mut c: ResMut<crate::combat::Combat>,
) {
    if h.open || f.vehicle == Vehicle::Spacecraft {
        return;
    }
    if keys.just_pressed(KeyCode::KeyY)
        && f.aircraft == Aircraft::F35B
        && !(keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
    {
        if f.airspeed < 150. {
            f.systems.conversion_target = !f.systems.conversion_target;
            f.status = if f.systems.conversion_target {
                "STOVL CONVERSION / Y TO RETURN"
            } else {
                "CONVENTIONAL FLIGHT"
            }
            .into();
        } else {
            f.status = "STOVL INHIBITED / REDUCE BELOW 150 m/s".into();
        }
    }
    if (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
        && keys.just_pressed(KeyCode::KeyJ)
    {
        f.systems.stores = [false; 10];
        c.ammo = [0; 2];
        f.status = "STORES JETTISONED".into();
    }
    if f.aircraft == Aircraft::Gripen {
        return;
    }
    if matches!(f.aircraft, Aircraft::F22 | Aircraft::F35B) {
        if c.active {
            f.systems.bays_target = f.systems.manual_bays || c.lock > 0.15;
        }
        if keys.just_pressed(KeyCode::KeyB)
            && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
        {
            f.systems.manual_bays = !f.systems.manual_bays;
            f.systems.bays_target = f.systems.manual_bays;
        }
    }
    if c.active && f.systems.weapon_generation != v.generation {
        let s = f.aircraft.spec();
        c.ammo = s.ammo;
        if let Some(fight) = &mut c.fight {
            fight.gun = s.gun;
        }
        f.systems.stores = std::array::from_fn(|i| i < (s.ammo[0] + s.ammo[1]) as usize);
        f.systems.weapon_generation = v.generation;
    }
    if c.active {
        let s = f.aircraft.spec();
        f.systems.stores = std::array::from_fn(|i| {
            if i < s.ammo[0] as usize {
                i >= s.ammo[0].saturating_sub(c.ammo[0]) as usize
            } else {
                i < (s.ammo[0] + s.ammo[1]) as usize
                    && i >= s.ammo[0] as usize + s.ammo[1].saturating_sub(c.ammo[1]) as usize
            }
        });
    }
}
