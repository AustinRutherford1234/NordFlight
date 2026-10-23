#[cfg(test)]
use crate::orbit::Orbit;
use crate::physics::*;
use bevy::{
    input::{ButtonState, keyboard::KeyboardInput},
    math::{DQuat, DVec3},
    prelude::*,
};
#[derive(Resource)]
pub struct Warp {
    pub rates: Vec<f64>,
    pub selected: usize,
    pub effective: f64,
    pub editing: bool,
    pub open: bool,
    pub buffer: String,
    pub message: String,
    pub timestep: f64,
    edit_timestep: bool,
}
impl Default for Warp {
    fn default() -> Self {
        Self {
            rates: vec![
                0.1, 0.25, 0.5, 0.75, 1.0, 2.0, 4.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 10000., 100000.,
            ],
            selected: 4,
            effective: 1.0,
            editing: false,
            open: false,
            buffer: String::new(),
            message: String::new(),
            timestep: 1.0 / 60.0,
            edit_timestep: false,
        }
    }
}
impl Warp {
    pub fn normal(&mut self) {self.selected=self.rates.iter().position(|r|*r==1.).unwrap_or(0);}
    fn path() -> std::path::PathBuf {
        let base = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        base.join(".nord-flight").join("settings.json")
    }
    pub fn parse(text: &str) -> Result<Vec<f64>, String> {
        let mut rates = vec![1.0];
        for part in text.split([',', ' ', ';']).filter(|s| !s.is_empty()) {
            let value = part
                .parse::<f64>()
                .map_err(|_| "Use numbers separated by commas".to_string())?;
            if !value.is_finite() || !(0.1..=100000.0).contains(&value) {
                return Err("Rates must be between 0.1 and 100000".into());
            }
            rates.push(value);
        }
        if rates.len() > 25 {
            return Err("Use at most 24 rates".into());
        }
        rates.sort_by(f64::total_cmp);
        rates.dedup();
        Ok(rates)
    }
    pub fn load() -> Self {
        let mut out = Self::default();
        if let Ok(text) = std::fs::read_to_string(Self::path())
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(values) = value["warp_rates"].as_array()
        {
            let text = values
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",");
            if let Ok(rates) = Self::parse(&text) {
                out.rates = rates;
            }
            if value["space_warp_version"].as_u64().unwrap_or(0) < 1 {
                for rate in [10000., 100000.] {
                    if !out.rates.contains(&rate) && out.rates.len() < 16 {
                        out.rates.push(rate);
                    }
                }
                out.rates.sort_by(f64::total_cmp);
            }
            if value["slow_motion_version"].as_u64().unwrap_or(0)<1 {
                for rate in [0.1,0.25,0.5,0.75] {if !out.rates.contains(&rate){out.rates.push(rate);}}
                out.rates.sort_by(f64::total_cmp);
            }
            if let Some(dt) = value["timestep"]
                .as_f64()
                .filter(|x| x.is_finite() && (0.001..=0.05).contains(x))
            {
                out.timestep = dt;
            }
        }
        out.normal();out
    }
    fn save(&mut self) {
        let path = Self::path();
        let result = std::fs::create_dir_all(path.parent().unwrap()).and_then(|_| {
            std::fs::write(
                &path,
                serde_json::json!({"warp_rates":self.rates,"timestep":self.timestep,"space_warp_version":1,"slow_motion_version":1}).to_string(),
            )
        });
        self.message = match result {
            Ok(()) => "Rates saved locally".into(),
            Err(e) => format!("Could not save: {e}"),
        };
    }
    pub fn allowed(&self, f: &Flight) -> f64 {
        let requested = self.rates[self.selected.min(self.rates.len() - 1)];
        if f.crashed || f.landed || f.agl() < 100.0 {
            return requested.min(1.0);
        }
        let safe_alt = if f.space.body == crate::space::Body::Moon {
            22000.
        } else {
            152000.
        };
        let coasting = f.vehicle == Vehicle::Spacecraft
            && f.agl() > safe_alt
            && f.throttle == 0.0
            && f.thrust < 1.0
            && f.controls.length_squared() < 1e-6
            && f.rates.length() < 0.01;
        if coasting {
            requested
        } else {
            requested.min(if f.vehicle == Vehicle::Gripen {
                2.0
            } else {
                4.0
            })
        }
    }
}
/// Spacecraft use bounded patched-conic universal-variable drifts. The legacy
/// Earth-only fallback retains central gravity and solar tide for other callers.
pub fn coast(f: &mut Flight, e: &mut Environment, duration: f64) -> f64 {
    let _measure = crate::performance::span(15);
    if f.vehicle == Vehicle::Spacecraft {
        return crate::space::coast(f, e, duration);
    }
    let omega = rotation();
    let mut remaining = duration;
    while remaining > 1e-9 {
        let dt = remaining.min(2.0);
        let mut r = f.position;
        let mut v = f.velocity + omega.cross(r);
        let acceleration = |p: DVec3, env: &Environment| {
            let sun = env.sun_direction() * env.sun_distance();
            let ds = sun - p;
            -p * MU / p.length().powi(3)
                + SUN_MU * (ds / ds.length().powi(3) - sun / sun.length().powi(3))
        };
        let a = acceleration(r, e);
        r += v * dt + a * 0.5 * dt * dt;
        e.advance(dt);
        v += (a + acceleration(r, e)) * 0.5 * dt;
        let q = DQuat::from_axis_angle(omega.normalize(), -omega.length() * dt);
        f.position = q * r;
        f.velocity = q * v - omega.cross(f.position);
        f.orientation = (q * f.orientation).normalize();
        f.elapsed += dt;
        f.spool *= (-dt / 0.3).exp();
        f.thrust = 0.0;
        f.rcs_torque = DVec3::ZERO;
        remaining -= dt;
    }
    let (t, p, rho, sound) = atmosphere(f.altitude());
    f.temperature = t;
    f.pressure = p;
    f.density = rho;
    f.airspeed = f.velocity.length();
    f.mach = f.airspeed / sound;
    f.status = "ORBITAL COAST / TIME WARP".into();
    duration
}
#[derive(Component)]
pub struct Action(u8);
#[derive(Component)]
pub struct Label(u8);
#[derive(Component)]
pub struct Panel;
pub fn setup(mut c: Commands) {
    for (id, label, x, w) in [
        (0, "−", 24.0, 32.0),
        (1, "+", 62.0, 32.0),
        (2, "1×", 100.0, 40.0),
        (3, "WARP SETTINGS", 146.0, 140.0),
    ] {
        c.spawn((
            Button,
            Action(id),
            crate::Chrome,
            Node {
                position_type: PositionType::Absolute,
                top: px(130),
                left: px(x),
                width: px(w),
                height: px(30),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.06, 0.09, 0.9)),
            ZIndex(45),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(label),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.9, 0.94)),
            ));
        });
    }
    c.spawn((
        Text::new(""),
        Label(0),
        crate::Chrome,
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.9, 0.94)),
        Node {
            position_type: PositionType::Absolute,
            top: px(138),
            left: px(300),
            ..default()
        },
        ZIndex(45),
    ));
    c.spawn((Node{position_type:PositionType::Absolute,top:px(173),left:px(24),width:px(590),padding:UiRect::all(px(18)),flex_direction:FlexDirection::Column,row_gap:px(14),..default()},Panel,Visibility::Hidden,BackgroundColor(Color::srgba(0.015,0.03,0.05,0.97)),ZIndex(60)))
        .with_children(|p| {
            p.spawn((Text::new("TIME WARP / SAVED RATES\nClick the rates to edit. Enter saves; Escape cancels.\nComma / period change rate. Slash returns to 1×."),TextFont{font_size:16.0,..default()},TextColor(Color::srgb(0.7,0.9,0.94))));
            p.spawn((Button,Action(4),Node{padding:UiRect::all(px(12)),min_height:px(42),..default()},BackgroundColor(Color::srgb(0.08,0.13,0.18))))
                .with_children(|b| {b.spawn((Text::new(""),Label(1),TextFont{font_size:17.0,..default()},TextColor(Color::WHITE)));});
            p.spawn((Text::new(""),Label(2),TextFont{font_size:14.0,..default()},TextColor(Color::srgb(1.0,0.7,0.3))));
            p.spawn((Button,Action(5),Node{padding:UiRect::all(px(12)),min_height:px(42),..default()},BackgroundColor(Color::srgb(0.08,0.13,0.18))))
                .with_children(|b|{b.spawn((Text::new(""),Label(3),TextFont{font_size:16.0,..default()},TextColor(Color::WHITE)));});
            p.spawn((Text::new("One configured step per frame; no realtime catch-up.\nTimestep range: 0.001–0.05 seconds. Enter saves."),TextFont{font_size:14.0,..default()},TextColor(Color::srgb(0.7,0.85,0.9))));
        });
}
pub fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    menu: Option<Res<crate::combat::bvr::Menu>>,
    mut events: MessageReader<KeyboardInput>,
    mut warp: ResMut<Warp>,
    buttons: Query<(&Action, &Interaction), Changed<Interaction>>,
) {
    if menu.is_some_and(|menu| menu.open) {
        events.clear();
        return;
    }
    for (action, i) in &buttons {
        if *i == Interaction::Pressed {
            match action.0 {
                0 => warp.selected = warp.selected.saturating_sub(1),
                1 => warp.selected = (warp.selected + 1).min(warp.rates.len() - 1),
                2 => warp.normal(),
                3 => {
                    warp.open = !warp.open;
                    warp.editing = false;
                }
                _ => {
                    warp.editing = true;
                    warp.edit_timestep = action.0 == 5;
                    warp.buffer = if warp.edit_timestep {
                        format!("{:.6}", warp.timestep)
                    } else {
                        warp.rates
                            .iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                }
            }
        }
    }
    if warp.editing {
        for event in events.read() {
            if event.state != ButtonState::Pressed {
                continue;
            }
            match event.key_code {
                KeyCode::Escape => warp.editing = false,
                KeyCode::Backspace => {
                    warp.buffer.pop();
                }
                KeyCode::Delete => warp.buffer.clear(),
                KeyCode::Enter if warp.edit_timestep => {
                    if let Ok(dt) = warp.buffer.parse::<f64>()
                        && dt.is_finite()
                        && (0.001..=0.05).contains(&dt)
                    {
                        warp.timestep = dt;
                        warp.editing = false;
                        warp.save();
                    } else {
                        warp.message = "Timestep must be 0.001–0.05 seconds".into();
                    }
                }
                KeyCode::Enter => match Warp::parse(&warp.buffer) {
                    Ok(rates) => {
                        warp.rates = rates;
                        warp.normal();
                        warp.editing = false;
                        warp.save();
                    }
                    Err(e) => warp.message = e,
                },
                _ => {
                    if let Some(text) = &event.text {
                        for ch in text
                            .chars()
                            .filter(|c| c.is_ascii_digit() || ",. ;".contains(*c))
                        {
                            if warp.buffer.len() < 180 {
                                warp.buffer.push(ch);
                            }
                        }
                    }
                }
            }
        }
    } else {
        events.clear();
        if keys.just_pressed(KeyCode::Comma) {
            warp.selected = warp.selected.saturating_sub(1);
        }
        if keys.just_pressed(KeyCode::Period) {
            warp.selected = (warp.selected + 1).min(warp.rates.len() - 1);
        }
        if keys.just_pressed(KeyCode::Slash) {
            warp.normal();
        }
    }
}
pub fn ui(
    warp: Res<Warp>,
    f: Res<Flight>,
    mut labels: Query<(&Label, &mut Text)>,
    mut panels: Query<&mut Visibility, With<Panel>>,
) {
    for mut v in &mut panels {
        v.set_if_neq(if warp.open {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    for (tag, mut t) in &mut labels {
        if tag.0 != 0 && !warp.open {
            continue;
        }
        let updated = match tag.0 {
            0 => format!(
                "WARP {}× / REQUESTED {}×{}",
                warp.effective,
                warp.rates[warp.selected],
                if f.paused {
                    " / PAUSED"
                } else if warp.effective < warp.rates[warp.selected] {
                    " / PHYSICS LIMIT"
                } else {
                    ""
                }
            ),
            1 => {
                if warp.editing && !warp.edit_timestep {
                    format!("{}▏", warp.buffer)
                } else {
                    warp.rates
                        .iter()
                        .map(|x| format!("{x}×"))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            }
            3 => {
                if warp.editing && warp.edit_timestep {
                    format!("TIMESTEP / {}▏ seconds", warp.buffer)
                } else {
                    format!("TIMESTEP / {:.6} seconds per frame", warp.timestep)
                }
            }
            _ => warp.message.clone(),
        };
        if t.0 != updated {
            t.0 = updated;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comma_below_normal_time_and_reset_use_actual_one_times_index() {
        let mut w=Warp::default();assert_eq!(w.rates[w.selected],1.);
        w.selected=w.selected.saturating_sub(1);
        assert_eq!(w.allowed(&Flight::runway()),0.75);
        w.selected=0;assert_eq!(w.allowed(&Flight::runway()),0.1);
        w.normal();assert_eq!(w.rates[w.selected],1.);
        assert_eq!(w.timestep,1./60.);
        assert_eq!(Warp::parse("0.1,0.5,2").unwrap(),vec![0.1,0.5,1.,2.]);
        assert!(Warp::parse("0.01").is_err());
    }
    #[test]
    fn rates_are_bounded_sorted_and_include_normal_time() {
        assert_eq!(
            Warp::parse("1000, 10, 2, 10").unwrap(),
            vec![1.0, 2.0, 10.0, 1000.0]
        );
        for s in ["NaN", "100001", "0", "-1"] {
            assert!(Warp::parse(s).is_err());
        }
    }
    #[test]
    fn warp_is_limited_near_ground_or_under_thrust() {
        let mut w = Warp::default();
        w.selected = w.rates.iter().position(|r|*r==1000.).unwrap();
        let mut f = Flight::spacecraft();
        assert_eq!(w.allowed(&f), 1000.0);
        f.throttle = 0.2;
        assert_eq!(w.allowed(&f), 4.0);
        assert_eq!(w.allowed(&Flight::runway()), 1.0);
    }
    #[test]
    fn lunar_and_interplanetary_warp_rates_allow_100000_only_on_safe_coasts() {
        let mut w = Warp::default();
        w.selected = w.rates.len() - 1;
        assert_eq!(
            Warp::parse("100000,10000,1").unwrap(),
            vec![1., 10000., 100000.]
        );
        let mut f = crate::space::scenario(9);
        assert_eq!(w.allowed(&f), 100000.);
        f.throttle = 0.1;
        assert_eq!(w.allowed(&f), 4.);
        assert_eq!(w.allowed(&crate::space::scenario(10)), 4.);
        f = crate::space::scenario(9);
        f.landed = true;
        assert_eq!(w.allowed(&f), 1.);
    }
    #[test]
    fn high_warp_coast_preserves_orbit_and_advances_both_clocks() {
        let mut f = Flight::spacecraft();
        let mut e = Environment::default();
        let before = Orbit::from_flight(&f);
        coast(&mut f, &mut e, 3600.0);
        let after = Orbit::from_flight(&f);
        assert!((f.elapsed - 3600.0).abs() < 1e-6 && (e.time - 3600.0).abs() < 1e-6);
        assert!((after.periapsis - before.periapsis).abs() < 100.0);
        assert!((after.apoapsis.unwrap() - before.apoapsis.unwrap()).abs() < 100.0);
    }
}
