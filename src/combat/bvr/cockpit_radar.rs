//! Shared cockpit/map radar commands and modifier-gated physical MFD picking.
use super::*;
use crate::{FlightCamera, ViewState, world};
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Command {
    Power,
    RangeUp,
    RangeDown,
    Mode,
    ElevationUp,
    ElevationDown,
    Next,
    Lock,
    Unlock,
    Page,
}
#[derive(Component)]
pub struct Bezel(pub Command);
/// Shared by the raised cap, label face, and ray picking.
pub const BUTTON_SIZE: Vec3 = Vec3::new(0.024, 0.034, 0.012);
pub const BUTTON_FACE_Z: f64 = -3.413;
#[derive(Resource, Default)]
pub struct Display {
    pub tactical: bool,
    pub hint: String,
    pub hover: Option<Command>,
    pub revision: u64,
}
#[derive(Component)]
pub struct Hint;
struct SensorView<'a> {
    radar: &'a Radar,
    enemies: usize,
    shots: &'a [Shot],
}
fn sensor(c: &Combat) -> SensorView<'_> {
    c.bvr.as_ref().map_or(
        SensorView {
            radar: &c.radar,
            enemies: c.targets.len(),
            shots: &[],
        },
        |b| SensorView {
            radar: &b.radar,
            enemies: b.enemies,
            shots: &b.shots,
        },
    )
}
pub fn radar(c: &Combat) -> &Radar {
    sensor(c).radar
}
fn radar_range(c: &Combat) -> f64 {
    if c.bvr.is_some() {
        RANGES[radar(c).range]
    } else {
        LOCAL_RANGES[radar(c).range]
    }
}
pub fn setup(c: &mut Commands) {
    c.spawn((
        Hint,
        Text::new(""),
        TextFont {
            font_size: 14.,
            ..default()
        },
        TextColor(Color::srgb(0.65, 1., 0.84)),
        Node {
            display: bevy::prelude::Display::None,
            position_type: PositionType::Absolute,
            left: px(28),
            bottom: px(58),
            padding: UiRect::all(px(7)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.015, 0.035, 0.045)),
        GlobalZIndex(57),
    ));
}
pub fn visuals(
    display: Res<Display>,
    combat: Res<Combat>,
    mouse: Res<ButtonInput<MouseButton>>,
    f: Res<Flight>,
    view: Res<ViewState>,
    menu: Res<Menu>,
    mut hint: Query<(&mut Text, &mut Node), With<Hint>>,
    mut buttons: Query<(&Bezel, &mut Transform, &MeshMaterial3d<StandardMaterial>, &crate::aircraft::visual::AircraftPart)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (mut text, mut node) in &mut hint {
        node.display = if f.vehicle == Vehicle::Gripen && view.index == 3 && !view.map && !menu.open
        {
            bevy::prelude::Display::Flex
        } else {
            bevy::prelude::Display::None
        };
        if text.0 != display.hint {
            text.0.clone_from(&display.hint);
        }
    }
    for (button, mut transform, material, aircraft) in &mut buttons {
        if aircraft.0!=f.aircraft {continue;}
        let hover = display.hover == Some(button.0);
        let active = illuminated(button.0, &combat, display.tactical);
        let z = -3.42
            - if hover && mouse.pressed(MouseButton::Left) {
                0.002
            } else {
                0.
            };
        if transform.translation.z != z {
            transform.translation.z = z;
        }
        {
            let color = if hover {
                Color::srgb(0.12, 0.48, 0.48)
            } else if active {
                Color::srgb(0.08, 0.25, 0.27)
            } else {
                Color::srgb(0.035, 0.065, 0.085)
            };
            let emissive = if hover {
                LinearRgba::new(0.08, 0.5, 0.45, 1.)
            } else if active {
                LinearRgba::new(0.02, 0.16, 0.15, 1.)
            } else {
                LinearRgba::BLACK
            };
            crate::render_cache::optics(&mut materials, &material.0, color, emissive);
        }
    }
}
fn illuminated(action: Command, c: &Combat, tactical: bool) -> bool {
    match action {
        Command::Power => radar(c).powered,
        Command::Lock => radar(c).powered && radar(c).mode == RadarMode::Stt,
        Command::Unlock => radar(c).powered && radar(c).mode == RadarMode::Tws,
        Command::Page => tactical,
        _ => false,
    }
}
/// A separate high-contrast face avoids unreadable labels painted onto the scope.
pub fn face_pixels(label: &str) -> Vec<u8> {
    let mut glyphs = vec![0; 256 * 192 * 4];
    crate::cockpit::radar_label(
        &mut glyphs,
        label,
        128 - label.len() as i32 * 3,
        93,
        [170, 255, 225, 255],
    );
    let mut result = vec![0; glyphs.len()];
    for y in 0..192_i32 {
        for x in 0..256_i32 {
            let sx = 128 + (x - 128).div_euclid((240 / (label.len().max(4) as i32 * 6)).max(1));
            let sy = 96 + (y - 96).div_euclid(8);
            let source = ((sy * 256 + sx) * 4) as usize;
            let dest = ((y * 256 + x) * 4) as usize;
            let ink = glyphs[source + 3] != 0;
            result[dest..dest + 4].copy_from_slice(if ink {
                &glyphs[source..source + 4]
            } else {
                &[8, 19, 25, 255]
            });
        }
    }
    result
}
pub fn label(action: Command) -> &'static str {
    match action {
        Command::Power => "PWR",
        Command::RangeUp => "RNG+",
        Command::RangeDown => "RNG-",
        Command::Mode => "MODE",
        Command::ElevationUp => "EL+",
        Command::ElevationDown => "EL-",
        Command::Next => "NEXT",
        Command::Lock => "LOCK",
        Command::Unlock => "UNLK",
        Command::Page => "PAGE",
    }
}
fn help(action: Command) -> &'static str {
    match action {
        Command::Power => "RADAR POWER ON / OFF",
        Command::RangeUp => "INCREASE SCAN RANGE",
        Command::RangeDown => "DECREASE SCAN RANGE",
        Command::Mode => "CYCLE SEARCH / TWS / SINGLE-TARGET TRACK",
        Command::ElevationUp => "RAISE SCAN ELEVATION 5 DEG",
        Command::ElevationDown => "LOWER SCAN ELEVATION 5 DEG",
        Command::Next => "SELECT NEXT DETECTED CONTACT",
        Command::Lock => "LOCK SELECTED CONTACT (STT)",
        Command::Unlock => "UNLOCK / RETURN TO TWS",
        Command::Page => "SWITCH RANGE-AZIMUTH / TACTICAL PLAN",
    }
}
pub fn bezel() -> Vec<(Command, Vec3)> {
    let mut result = Vec::new();
    for (side, actions) in [
        (
            -1.,
            [
                Command::Power,
                Command::RangeUp,
                Command::RangeDown,
                Command::Mode,
            ],
        ),
        (
            1.,
            [
                Command::ElevationUp,
                Command::ElevationDown,
                Command::Next,
                Command::Lock,
            ],
        ),
    ] {
        for (i, action) in actions.into_iter().enumerate() {
            result.push((
                action,
                Vec3::new(0.29 + side * 0.135, 0.590 - i as f32 * 0.040, -3.42),
            ));
        }
    }
    result.push((Command::Page, Vec3::new(0.23, 0.365, -3.42)));
    result.push((Command::Unlock, Vec3::new(0.35, 0.365, -3.42)));
    result
}
pub fn command(c: &mut Combat, action: Command) {
    if action == Command::Next {
        if c.bvr.is_some() {
            c.cycle();
        } else {
            for offset in 1..=c.targets.len() {
                let index = (c.selected + offset) % c.targets.len();
                if c.targets[index].alive
                    && c.radar
                        .tracks
                        .get(index)
                        .is_some_and(|tr| tr.valid(c.clock))
                {
                    c.selected = index;
                    break;
                }
            }
            c.lock = 0.;
        }
        return;
    }
    let r = if let Some(b) = &mut c.bvr {
        &mut b.radar
    } else {
        &mut c.radar
    };
    match action {
        Command::Power => {
            r.powered = !r.powered;
            c.lock = 0.;
        }
        Command::RangeUp => r.range = (r.range + 1).min(3),
        Command::RangeDown => r.range = r.range.saturating_sub(1),
        Command::Mode => {
            r.mode = match r.mode {
                RadarMode::Search => RadarMode::Tws,
                RadarMode::Tws => RadarMode::Stt,
                RadarMode::Stt => RadarMode::Search,
            };
            c.lock = 0.;
        }
        Command::ElevationUp => r.elevation = (r.elevation + 5.).min(45.),
        Command::ElevationDown => r.elevation = (r.elevation - 5.).max(-45.),
        Command::Lock => {
            r.mode = RadarMode::Stt;
            c.lock = 0.;
        }
        Command::Unlock => {
            r.mode = RadarMode::Tws;
            c.lock = 0.;
        }
        _ => {}
    }
}
fn track_point(c: &Combat, f: &Flight, index: usize, tactical: bool) -> Option<Vec2> {
    let b = sensor(c);
    let tr = b.radar.tracks.get(index)?;
    if index >= b.enemies || !tr.valid(c.clock) {
        return None;
    }
    let offset = tr.position + tr.velocity * (c.clock - tr.seen) - f.position;
    let relative = f.orientation.conjugate() * offset;
    let range = radar_range(c);
    let point = if tactical {
        let up = f.up();
        let ahead = (f.forward() - up * f.forward().dot(up)).normalize_or_zero();
        let right = ahead.cross(up).normalize_or_zero();
        Vec2::new(
            128. + (offset.dot(right) / range * 90.) as f32,
            100. - (offset.dot(ahead) / range * 65.) as f32,
        )
    } else {
        let az = relative.x.atan2(-relative.z).to_degrees();
        if az.abs() > 60. {
            return None;
        }
        Vec2::new(
            128. + (az / 60. * 88.) as f32,
            153. - (relative.length() / range * 108.) as f32,
        )
    };
    ((42. ..214.).contains(&point.x) && (40. ..156.).contains(&point.y)).then_some(point)
}
fn plane_point(origin: DVec3, direction: DVec3, z: f64) -> Option<DVec3> {
    if direction.z.abs() < 1e-9 {
        return None;
    }
    let distance = (z - origin.z) / direction.z;
    (distance > 0.).then_some(origin + direction * distance)
}
fn pick_bezel(origin: DVec3, direction: DVec3) -> Option<Command> {
    let p = plane_point(origin, direction, BUTTON_FACE_Z)?;
    bezel()
        .into_iter()
        .find(|(_, center)| {
            (p.x - center.x as f64).abs() <= BUTTON_SIZE.x as f64 * 0.5 + 0.001
                && (p.y - center.y as f64).abs() <= BUTTON_SIZE.y as f64 * 0.5 + 0.001
        })
        .map(|(action, _)| action)
}
fn screen_pixel(origin: DVec3, direction: DVec3) -> Option<Vec2> {
    let p = plane_point(origin, direction, -3.44)?;
    let uv = Vec2::new(
        ((p.x - 0.29) / 0.245 + 0.5) as f32,
        (0.5 - (p.y - 0.49) / 0.205) as f32,
    );
    ((0. ..=1.).contains(&uv.x) && (0. ..=1.).contains(&uv.y)).then_some(uv * Vec2::new(256., 192.))
}
pub fn select(c: &mut Combat, f: &Flight, pixel: Vec2, tactical: bool) -> bool {
    if !f.battery || f.damage.radar < 0.25 || !radar(c).powered {
        return false;
    }
    let index = (0..c.targets.len())
        .filter_map(|i| track_point(c, f, i, tactical).map(|p| (i, p.distance(pixel))))
        .filter(|(_, r)| *r < 12.)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|p| p.0);
    if let Some(index) = index {
        c.selected = index;
        c.lock = 0.;
        true
    } else {
        false
    }
}

#[derive(Debug)]
struct HudTrack {
    index: usize,
    point: Vec2,
    edge: bool,
    behind: bool,
    selected: bool,
    friendly: bool,
    coasting: bool,
}
fn hud_tracks(c: &Combat, f: &Flight, simple: bool) -> Vec<HudTrack> {
    let b = sensor(c);
    if !b.radar.powered || !f.battery || f.damage.radar < 0.25 {
        return Vec::new();
    }
    let min = Vec2::new(60., 43.);
    let max = Vec2::new(196., 139.);
    let mut marks = Vec::new();
    for (index, tr) in b
        .radar
        .tracks
        .iter()
        .enumerate()
        .filter(|(_, tr)| tr.valid(c.clock))
    {
        let selected = index == c.selected && index < b.enemies;
        if simple && !selected {
            continue;
        }
        let offset = tr.position + tr.velocity * (c.clock - tr.seen) - f.position;
        let local = (f.orientation.conjugate() * offset).normalize_or_zero();
        let projection = crate::cockpit::hud_projection(f.vehicle, local.as_vec3());
        let behind = local.z >= 0.;
        let raw = projection.unwrap_or_else(|| {
            Vec2::new(
                if local.x.abs() > 0.05 {
                    if local.x > 0. { 256. } else { 0. }
                } else {
                    128.
                },
                if local.y > 0.05 { 0. } else { 192. },
            )
        });
        let point = raw.clamp(min, max);
        let edge = projection.is_none() || point.distance(raw) > 0.5;
        // Only the designation receives an off-HUD steering cue.
        if edge && !selected {
            continue;
        }
        marks.push(HudTrack {
            index,
            point,
            edge,
            behind,
            selected,
            friendly: index >= b.enemies,
            coasting: c.clock - tr.seen > 3.,
        });
    }
    marks.sort_by_key(|m| m.selected);
    marks
}
pub fn selected_hud_line(c: &Combat, f: &Flight) -> Option<String> {
    let b = sensor(c);
    if !b.radar.powered || !f.battery || f.damage.radar < 0.25 {
        return None;
    }
    let tr = b
        .radar
        .tracks
        .get(c.selected)
        .filter(|tr| tr.valid(c.clock))?;
    let r = tr.position + tr.velocity * (c.clock - tr.seen) - f.position;
    Some(if c.clock - tr.seen > 3. {
        format!("T{:02} {:.0}KM COAST", c.selected + 1, r.length() / 1000.)
    } else {
        format!(
            "T{:02} {:.0}KM VC{:+.0}",
            c.selected + 1,
            r.length() / 1000.,
            (f.velocity - tr.velocity).dot(r.normalize_or_zero())
        )
    })
}
#[derive(Debug)]
struct LaunchZone {
    range: f64,
    max: f64,
    nez: f64,
    coasting: bool,
}
fn launch_zone(c: &Combat, f: &Flight) -> Option<LaunchZone> {
    let b = sensor(c);
    if !b.radar.powered || !f.battery || f.damage.radar < 0.25 || c.selected >= b.enemies {
        return None;
    }
    let tr = b
        .radar
        .tracks
        .get(c.selected)
        .filter(|tr| tr.valid(c.clock))?;
    let position = tr.position + tr.velocity * (c.clock - tr.seen);
    let offset = position - f.position;
    let (max, nez) = if c.weapon == Weapon::Radar && c.bvr.is_some() {
        envelope(
            f.position,
            f.velocity,
            Contact {
                position,
                velocity: tr.velocity,
                forward: -tr.velocity.normalize_or_zero(),
                throttle: 1.,
                engine: true,
                alive: true,
                decoy: false,
                strength: 1.,
            },
        )
    } else {
        let closure = (f.velocity - tr.velocity).dot(offset.normalize_or_zero());
        let factor =
            (0.55 + f.altitude().max(0.) / 30000. + f.velocity.length() / 1200. + closure / 1600.)
                .clamp(0.35, 1.);
        let max = c.weapon.range() * factor;
        (max, max * 0.48)
    };
    Some(LaunchZone {
        range: offset.length(),
        max,
        nez,
        coasting: c.clock - tr.seen > 3.,
    })
}
fn draw_zone(data: &mut [u8], zone: &LaunchZone, x: i32, top: i32, bottom: i32) {
    use crate::cockpit::{radar_label as text, radar_line as line};
    let scale = (zone.max * 1.25).max(zone.range * 1.10);
    let y = |range: f64| {
        bottom - ((range / scale).clamp(0., 1.) * (bottom - top) as f64).round() as i32
    };
    let dim = [45, 125, 105, 255];
    let green = [100, 245, 165, 255];
    let amber = [255, 190, 65, 255];
    let color = if zone.coasting {
        [130, 165, 145, 220]
    } else if zone.range <= zone.nez && zone.range >= 500. {
        green
    } else {
        amber
    };
    line(data, x, top, x, bottom, dim);
    for xx in x - 2..=x + 2 {
        line(data, xx, y(zone.nez), xx, y(500.), dim);
    }
    line(data, x - 5, y(zone.max), x + 5, y(zone.max), green);
    line(data, x - 5, y(zone.nez), x + 5, y(zone.nez), green);
    let ty = y(zone.range);
    line(data, x - 9, ty - 3, x - 4, ty, color);
    line(data, x - 9, ty + 3, x - 4, ty, color);
    text(
        data,
        &format!(
            "MAX{}",
            if zone.max < 10000. {
                format!("{:.1}", zone.max / 1000.)
            } else {
                format!("{:.0}", zone.max / 1000.)
            }
        ),
        x - 37,
        top - 9,
        green,
    );
    text(
        data,
        &format!(
            "NEZ{}",
            if zone.nez < 10000. {
                format!("{:.1}", zone.nez / 1000.)
            } else {
                format!("{:.0}", zone.nez / 1000.)
            }
        ),
        x - 37,
        bottom + 3,
        color,
    );
    text(data, "EST", x - 17, top + 3, dim);
}
pub fn draw_hud(data: &mut [u8], c: &Combat, f: &Flight, simple: bool) {
    use crate::cockpit::{radar_label as text, radar_line as line};
    for mark in hud_tracks(c, f, simple) {
        let (x, y) = (mark.point.x.round() as i32, mark.point.y.round() as i32);
        let color = if mark.coasting {
            [130, 165, 145, 220]
        } else if mark.selected {
            [255, 190, 65, 255]
        } else if mark.friendly {
            [85, 205, 235, 255]
        } else {
            [100, 245, 165, 255]
        };
        if mark.edge {
            let toward = (mark.point - Vec2::new(128., 91.)).normalize_or_zero();
            let base = mark.point - toward * 8.;
            let cross = Vec2::new(-toward.y, toward.x) * 4.;
            for end in [base + cross, base - cross] {
                line(data, x, y, end.x as i32, end.y as i32, color);
            }
            text(
                data,
                if mark.behind { "AFT" } else { "STEER" },
                (x - 12).clamp(61, 165),
                (y + 7).min(142),
                color,
            );
        } else if mark.friendly {
            for (a, b) in [
                ((-3, -3), (3, -3)),
                ((3, -3), (3, 3)),
                ((3, 3), (-3, 3)),
                ((-3, 3), (-3, -3)),
            ] {
                line(data, x + a.0, y + a.1, x + b.0, y + b.1, color);
            }
        } else {
            let size = if mark.selected { 6 } else { 3 };
            for (a, b) in [
                ((-size, 0), (0, -size)),
                ((0, -size), (size, 0)),
                ((size, 0), (0, size)),
                ((0, size), (-size, 0)),
            ] {
                line(data, x + a.0, y + a.1, x + b.0, y + b.1, color);
            }
            if mark.selected {
                let locked = radar(c).mode == RadarMode::Stt && !mark.coasting;
                text(
                    data,
                    &format!("{}{}", if locked { "L" } else { "T" }, mark.index + 1),
                    (x + 9).min(181),
                    if y < 60 { y + 10 } else { y - 15 },
                    color,
                );
            }
        }
        if mark.coasting && mark.selected {
            text(
                data,
                "COAST",
                (x - 12).clamp(61, 165),
                if mark.edge && y > 100 {
                    y - 19
                } else {
                    (y + 10).min(142)
                },
                color,
            );
        }
    }
    if let Some(zone) = launch_zone(c, f) {
        draw_zone(data, &zone, 186, 52, 128);
        if !zone.coasting && (500. ..=zone.nez).contains(&zone.range) {
            text(data, "IN NEZ EST", 8, 139, [100, 245, 165, 255]);
        }
    }
}
fn missile_symbols(c: &Combat, f: &Flight, tactical: bool) -> Vec<(Vec2, bool)> {
    let b = sensor(c);
    if !b.radar.powered || !f.battery || f.damage.radar < 0.25 {
        return vec![];
    }
    let scale = radar_range(c);
    let up = f.up();
    let ahead = (f.forward() - up * f.forward().dot(up)).normalize_or_zero();
    let right = ahead.cross(up).normalize_or_zero();
    c.missiles_for_draw()
        .enumerate()
        .filter(|(_, m)| m.alive)
        .filter_map(|(index, m)| {
            let own = c.bvr.as_ref().map_or(index < c.missiles.len(), |b| {
                b.shots.get(index).is_some_and(|s| s.friendly)
            });
            let offset = m.position - f.position;
            // Own/datalink missiles are known. Other missiles require a local warning:
            // finite range, closing motion, and terrain visibility.
            if !own
                && (offset.length() > 35000.
                    || (m.velocity - f.velocity).dot(-offset) <= 0.
                    || !guidance::terrain_visible(f.position, m.position))
            {
                return None;
            }
            let local = f.orientation.conjugate() * offset;
            let p = if tactical {
                Vec2::new(
                    128. + (offset.dot(right) / scale * 90.) as f32,
                    100. - (offset.dot(ahead) / scale * 65.) as f32,
                )
            } else {
                let az = local.x.atan2(-local.z).to_degrees();
                if az.abs() > 60. {
                    return None;
                }
                Vec2::new(
                    128. + (az / 60. * 88.) as f32,
                    153. - (offset.length() / scale * 108.) as f32,
                )
            };
            ((42. ..214.).contains(&p.x) && (40. ..156.).contains(&p.y)).then_some((p, own))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scene() -> (Combat, Flight) {
        let f = super::super::flight();
        let mut c = Combat::default();
        super::super::start(&mut c, &f, 1, &Menu::default());
        let b = c.bvr.as_mut().unwrap();
        for (i, t) in c.targets.iter().enumerate() {
            b.radar.tracks[i] = Track {
                position: t.position,
                velocity: t.velocity,
                seen: 0.,
                confidence: 1.,
                reports: 3,
                friendly_report: false,
            };
        }
        (c, f)
    }
    #[test]
    fn physical_button_and_screen_rays_match_rendered_geometry() {
        let eye = DVec3::new(0., 1.08, -2.65);
        for (action, center) in bezel() {
            assert_eq!(
                pick_bezel(eye, (center.as_dvec3() - eye).normalize()),
                Some(action)
            );
        }
        let direction = (DVec3::new(0.29, 0.49, -3.44) - eye).normalize();
        assert!(
            screen_pixel(eye, direction)
                .unwrap()
                .distance(Vec2::new(128., 96.))
                < 0.001
        );
        assert_eq!(pick_bezel(eye, DVec3::Z), None);
        assert_eq!(screen_pixel(eye, DVec3::Z), None);
        assert_eq!(screen_pixel(eye, DVec3::Y), None);
    }
    #[test]
    fn raised_button_faces_are_pickable_to_their_visible_corners() {
        let eye = DVec3::new(0., 1.08, -2.65);
        for (action, center) in bezel() {
            for x in [-1., 1.] {
                for y in [-1., 1.] {
                    let corner = DVec3::new(
                        center.x as f64 + x * BUTTON_SIZE.x as f64 * 0.499,
                        center.y as f64 + y * BUTTON_SIZE.y as f64 * 0.499,
                        BUTTON_FACE_Z,
                    );
                    assert_eq!(pick_bezel(eye, (corner - eye).normalize()), Some(action));
                }
            }
        }
        let gap = DVec3::new(0.155, 0.570, BUTTON_FACE_Z);
        assert_eq!(pick_bezel(eye, (gap - eye).normalize()), None);
    }
    #[test]
    fn bezel_lamps_follow_actual_radar_and_page_state() {
        let (mut c, _) = scene();
        assert!(illuminated(Command::Power, &c, false));
        command(&mut c, Command::Lock);
        assert!(illuminated(Command::Lock, &c, false));
        assert!(!illuminated(Command::Unlock, &c, false));
        command(&mut c, Command::Unlock);
        assert!(illuminated(Command::Unlock, &c, false));
        command(&mut c, Command::Power);
        assert!(!illuminated(Command::Power, &c, false));
        assert!(!illuminated(Command::Unlock, &c, false));
        assert!(illuminated(Command::Page, &c, true));
        assert!(!illuminated(Command::Page, &c, false));
    }
    #[test]
    fn tactical_scope_stays_level_during_roll() {
        let (c, mut f) = scene();
        let before = track_point(&c, &f, 0, true).unwrap();
        f.orientation *= bevy::math::DQuat::from_rotation_z(0.9);
        let after = track_point(&c, &f, 0, true).unwrap();
        assert!(before.distance(after) < 0.001);
    }
    #[test]
    fn both_mfd_pages_designate_the_clicked_contact() {
        for tactical in [false, true] {
            let (mut c, f) = scene();
            let p = track_point(&c, &f, 1, tactical).unwrap();
            assert!(select(&mut c, &f, p, tactical));
            assert_eq!(c.selected, 1);
            assert_eq!(c.lock, 0.);
        }
    }
    #[test]
    fn hud_uses_sensed_tracks_and_simplified_view_keeps_designation() {
        let (mut c, f) = scene();
        let all = hud_tracks(&c, &f, false);
        assert!(all.iter().any(|m| m.index == 1 && !m.selected));
        assert_eq!(hud_tracks(&c, &f, true).len(), 1);
        let before = all.iter().find(|m| m.selected).unwrap().point;
        // Moving target truth without a radar update must not move its symbol.
        c.targets[0].position += f.orientation * DVec3::X * 50000.;
        let after = hud_tracks(&c, &f, false)
            .into_iter()
            .find(|m| m.selected)
            .unwrap()
            .point;
        assert_eq!(before, after);
        let mut data = vec![0; 256 * 192 * 4];
        draw_hud(&mut data, &c, &f, false);
        assert!(data.chunks_exact(4).any(|p| p == [255, 190, 65, 255]));
        c.bvr.as_mut().unwrap().radar.powered = false;
        assert!(hud_tracks(&c, &f, false).is_empty());
        assert!(selected_hud_line(&c, &f).is_none());
    }
    #[test]
    fn hud_coasts_then_expires_and_gives_selected_offscreen_steering() {
        let (mut c, f) = scene();
        let tr = &mut c.bvr.as_mut().unwrap().radar.tracks[0];
        tr.position = f.position - f.forward() * 10000.;
        tr.velocity = DVec3::ZERO;
        let mark = hud_tracks(&c, &f, true).remove(0);
        assert!(mark.edge && mark.behind);
        c.clock = 4.;
        assert!(hud_tracks(&c, &f, true)[0].coasting);
        assert!(selected_hud_line(&c, &f).unwrap().contains("COAST"));
        c.clock = 18.1;
        assert!(hud_tracks(&c, &f, true).is_empty());
    }
    #[test]
    fn controls_share_live_radar_state_and_obey_limits() {
        let (mut c, f) = scene();
        command(&mut c, Command::Lock);
        assert_eq!(c.bvr.as_ref().unwrap().radar.mode, RadarMode::Stt);
        command(&mut c, Command::Unlock);
        assert_eq!(c.bvr.as_ref().unwrap().radar.mode, RadarMode::Tws);
        for _ in 0..20 {
            command(&mut c, Command::RangeUp);
            command(&mut c, Command::ElevationUp);
        }
        let b = c.bvr.as_ref().unwrap();
        assert_eq!(b.radar.range, 3);
        assert_eq!(b.radar.elevation, 45.);
        command(&mut c, Command::Power);
        assert_eq!(c.cue(&f), Err("RADAR STANDBY"));
        assert!(!select(&mut c, &f, Vec2::new(128., 70.), false));
    }
    #[test]
    fn shared_radar_acquires_practice_tracks_and_hides_unsensed_truth() {
        let mut f = super::super::super::flight();
        let mut c = Combat::default();
        c.start(&f, 1);
        c.targets[0].position = f.position + f.forward() * 3000.;
        c.targets[0].velocity = f.velocity;
        command(&mut c, Command::Lock);
        super::super::sense_local(&mut c, &f);
        assert!(radar(&c).tracks[0].valid(c.clock));
        assert!(hud_tracks(&c, &f, false).iter().any(|m| m.selected));
        assert!(selected_hud_line(&c, &f).is_some());
        assert!(launch_zone(&c, &f).unwrap().max <= 6000.);
        let p = track_point(&c, &f, 0, false).unwrap();
        assert!(select(&mut c, &f, p, false));
        command(&mut c, Command::Power);
        assert!(hud_tracks(&c, &f, false).is_empty());
        c.weapon = Weapon::Radar;
        assert_eq!(c.cue(&f), Err("RADAR STANDBY"));
        f.battery = false;
        assert!(!select(&mut c, &f, p, false));
    }
    #[test]
    fn launch_zone_uses_measured_motion_and_selected_weapon() {
        let (mut c, f) = scene();
        let long = launch_zone(&c, &f).unwrap();
        assert!(long.max > long.nez && long.nez > 0.);
        c.weapon = Weapon::Heat;
        let short = launch_zone(&c, &f).unwrap();
        assert!(short.max <= 6000. && short.max < long.max);
        c.clock = 4.;
        assert!(launch_zone(&c, &f).unwrap().coasting);
        c.clock = 19.;
        assert!(launch_zone(&c, &f).is_none());
    }
    #[test]
    fn missile_symbols_show_live_friendly_and_closing_threat_only() {
        let (mut c, mut f) = scene();
        f.paused = false;
        c.targets[0].position = f.position + f.forward() * 30000.;
        c.bvr.as_mut().unwrap().radar.tracks[0].position = c.targets[0].position;
        c.lock = c.weapon.lock_time();
        assert!(super::super::fire(&mut c, &f));
        c.missiles[0].position = f.position + f.forward() * 12000.;
        let mut hostile = c.missiles[0].clone();
        hostile.position = f.position + f.forward() * 20000. + f.orientation * DVec3::X * 2000.;
        hostile.velocity = -f.forward() * 600.;
        c.missiles.push(hostile);
        c.bvr.as_mut().unwrap().shots.push(Shot {
            launcher: Some(0),
            target: None,
            friendly: false,
        });
        let symbols = missile_symbols(&c, &f, false);
        assert_eq!(symbols.len(), 2);
        assert!(symbols.iter().any(|(_, own)| *own));
        assert!(symbols.iter().any(|(_, own)| !*own));
        c.missiles[1].velocity = f.forward() * 900.;
        assert_eq!(missile_symbols(&c, &f, false).len(), 1);
        c.missiles[0].alive = false;
        assert!(missile_symbols(&c, &f, false).is_empty());
    }
    #[test]
    fn raster_pages_include_tracks_and_selected_contact_data() {
        let (c, f) = scene();
        let mut scope = vec![0; 256 * 192 * 4];
        let mut tactical = scope.clone();
        draw(&mut scope, &c, &f, false);
        draw(&mut tactical, &c, &f, true);
        assert_ne!(scope, tactical);
        assert!(scope.chunks_exact(4).any(|p| p == [255, 185, 75, 255]));
    }
}
pub fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<FlightCamera>>,
    f: Res<Flight>,
    view: Res<ViewState>,
    menu: Res<Menu>,
    mut c: ResMut<Combat>,
    mut display: ResMut<Display>,
) {
    display.hover = None;
    if f.vehicle != Vehicle::Gripen
        || view.index != 3
        || view.map
        || view.missile_follow.is_some()
        || menu.open
    {
        return;
    }
    if !(keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight)) {
        display.hint =
            "OPTION / ALT + CLICK: COCKPIT SWITCHES / RADAR TRACKS  /  WHEEL: ZOOM  /  HOME: RECENTER".into();
        return;
    }
    let Some(cursor) = windows.single().ok().and_then(|w| w.cursor_position()) else {
        return;
    };
    let Some((camera, transform)) = cameras.iter().next() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(transform, cursor) else {
        return;
    };
    let origin = f.orientation.conjugate()
        * (world::render_origin(&f) + world::frame(&f) * ray.origin.as_dvec3() - f.position) - f.aircraft.cabin_offset().as_dvec3();
    let direction = f.orientation.conjugate() * (world::frame(&f) * ray.direction.as_dvec3());
    if let Some(action) = pick_bezel(origin, direction) {
        display.hover = Some(action);
        display.hint = format!("OPTION / ALT + CLICK: {}", help(action));
        if mouse.just_pressed(MouseButton::Left) {
            if action == Command::Page {
                display.tactical = !display.tactical;
            } else {
                command(&mut c, action);
            }
            display.revision = display.revision.wrapping_add(1);
        }
        return;
    }
    if let Some(pixel) = screen_pixel(origin, direction) {
        display.hint = "OPTION / ALT + CLICK: SELECT RADAR CONTACT".into();
        if mouse.just_pressed(MouseButton::Left) {
            select(&mut c, &f, pixel, display.tactical);
            display.revision = display.revision.wrapping_add(1);
        }
    }
}
pub fn draw(data: &mut [u8], c: &Combat, f: &Flight, tactical: bool) {
    let _measure = crate::performance::span(6);
    use crate::cockpit::{radar_label as text, radar_line as line};
    let green = [90, 245, 175, 255];
    let dim = [25, 85, 72, 255];
    let amber = [255, 185, 75, 255];
    let b = sensor(c);
    // Quiet scope rails; control labels now live on the actual raised keys.
    line(data, 3, 2, 3, 176, dim);
    line(data, 252, 2, 252, 176, dim);
    text(data, "RADAR CONTROL / OPT+CLICK", 38, 181, dim);
    text(
        data,
        &format!(
            "{} / {} {}",
            if tactical { "PLAN H-UP" } else { "RANGE-AZ" },
            if c.weapon == Weapon::Heat {
                "IR"
            } else if c.bvr.is_some() {
                "LR-AAM"
            } else {
                "RADAR"
            },
            c.ammo[c.weapon.index()]
        ),
        35,
        3,
        green,
    );
    text(
        data,
        &format!(
            "{} {}KM EL{:+.0}",
            b.radar.mode.label(),
            radar_range(c) / 1000.,
            b.radar.elevation
        ),
        38,
        15,
        green,
    );
    if !b.radar.powered || !f.battery || f.damage.radar < 0.25 {
        text(
            data,
            if !f.battery {
                "NO ELECTRICAL POWER"
            } else if f.damage.radar < 0.25 {
                "RADAR FAILURE"
            } else {
                "RADAR STANDBY"
            },
            44,
            90,
            amber,
        );
        return;
    }
    for i in 0..5 {
        let y = 45 + i * 27;
        line(data, 40, y, 216, y, dim);
    }
    for x in [40, 84, 128, 172, 216] {
        line(data, x, 45, x, 153, dim);
    }
    if !tactical {
        let x = 128 + (b.radar.scan / 60. * 88.) as i32;
        line(data, x, 45, x, 153, [40, 145, 100, 255]);
    } else {
        line(data, 122, 100, 134, 100, green);
        line(data, 128, 94, 128, 106, green);
    }
    for i in 0..b.enemies {
        if let Some(p) = track_point(c, f, i, tactical) {
            if !(42. ..214.).contains(&p.x) || !(40. ..156.).contains(&p.y) {
                continue;
            }
            let (x, y) = (p.x as i32, p.y as i32);
            let color = if i == c.selected { amber } else { green };
            line(data, x - 3, y, x, y - 3, color);
            line(data, x, y - 3, x + 3, y, color);
            line(data, x + 3, y, x, y + 3, color);
            line(data, x, y + 3, x - 3, y, color);
            if i == c.selected {
                line(data, x - 6, y - 6, x + 6, y - 6, amber);
                line(data, x - 6, y + 6, x + 6, y + 6, amber);
            }
            // Close formations can occupy the same scope pixels at long range.
            // Label only the designation; NEXT can step through the cluster.
            if i == c.selected {
                text(data, &format!("{}", i + 1), x + 8, y - 10, color);
            }
        }
    }
    for (p, own) in missile_symbols(c, f, tactical) {
        let (x, y) = (p.x as i32, p.y as i32);
        let color = if own {
            [100, 185, 255, 255]
        } else {
            [255, 85, 65, 255]
        };
        line(data, x, y - 4, x - 3, y + 3, color);
        line(data, x, y - 4, x + 3, y + 3, color);
        line(data, x - 3, y + 3, x + 3, y + 3, color);
    }
    if let Some(zone) = launch_zone(c, f) {
        draw_zone(data, &zone, 208, 52, 139);
        if !zone.coasting && (500. ..=zone.nez).contains(&zone.range) {
            text(data, "IN NEZ EST", 45, 142, green);
        }
    }
    if let Some(tr) = b.radar.tracks.get(c.selected).filter(|t| t.valid(c.clock)) {
        let estimate = tr.position + tr.velocity * (c.clock - tr.seen);
        let r = estimate - f.position;
        let local = f.orientation.conjugate() * r;
        let closure = (f.velocity - tr.velocity).dot(r.normalize_or_zero());
        text(
            data,
            &format!(
                "AZ{:+.0} VC{:+.0}",
                local.x.atan2(-local.z).to_degrees(),
                closure
            ),
            38,
            27,
            green,
        );
        let (max, nez) = launch_zone(c, f).map_or((0., 0.), |zone| (zone.max, zone.nez));
        text(
            data,
            &format!(
                "T{:02} {:.0}KM {:.1}K AGE{:.0}",
                c.selected + 1,
                r.length() / 1000.,
                (estimate.length() - RADIUS) / 1000.,
                c.clock - tr.seen
            ),
            38,
            160,
            amber,
        );
        text(
            data,
            &format!(
                "MAX{:.0} NEZ{:.0} {}",
                max / 1000.,
                nez / 1000.,
                if c.cue(f).is_ok() {
                    if c.ammo[c.weapon.index()] == 0 {
                        "EMPTY"
                    } else if f.paused {
                        "PAUSED"
                    } else if c.lock >= c.weapon.lock_time() {
                        "SHOOT"
                    } else {
                        "ACQ"
                    }
                } else {
                    match c.cue(f).unwrap_err() {
                        "OUTSIDE ESTIMATED RMAX" | "IR OUT OF RANGE" => "OUT RNG",
                        "SELECT TWS OR STT" => "SET TWS",
                        "NO FRESH RADAR TRACK" => "STALE",
                        "RADAR GIMBAL LIMIT" | "POINT NOSE AT TARGET" => "NOSE ON",
                        "TOO CLOSE" => "TOO NEAR",
                        _ => "INHIBIT",
                    }
                }
            ),
            38,
            170,
            green,
        );
    } else {
        text(data, "NO TRACK / SEARCH AHEAD", 40, 164, green);
    }
    let support = c
        .missiles
        .iter()
        .zip(b.shots.iter())
        .filter(|(m, s)| m.alive && !m.gnc.active && s.launcher.is_none())
        .count();
    text(data, &format!("SUP {}", support), 175, 27, amber);
    let count = missile_symbols(c, f, tactical).len();
    if count > 0 {
        text(data, &format!("MSL {}", count), 110, 181, green);
        text(data, "FRND MSL", 40, 37, [100, 185, 255, 255]);
        text(data, "THREAT", 103, 37, [255, 85, 65, 255]);
    }
}
