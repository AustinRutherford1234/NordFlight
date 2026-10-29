//! Original Super Hornet-style display. Simulation estimates, not an OFP replica.
use crate::{
    combat::{Combat, Weapon},
    physics::*,
};
use bevy::{math::DVec3, prelude::*};
pub const WIDTH: usize = 1024;
pub const HEIGHT: usize = 768;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MasterMode {
    #[default]
    Nav,
    Air,
    Ground,
}
impl MasterMode {
    pub fn next(self) -> Self {
        match self {
            Self::Nav => Self::Air,
            Self::Air => Self::Ground,
            Self::Ground => Self::Nav,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Nav => "NAV",
            Self::Air => "A/A",
            Self::Ground => "A/G",
        }
    }
}
#[derive(Clone, Debug)]
pub struct State {
    pub mode: MasterMode,
    pub reject: u8,
    pub caged: bool,
    pub radar_alt: bool,
    pub brightness: f32,
}
impl Default for State {
    fn default() -> Self {
        Self {
            mode: MasterMode::Nav,
            reject: 0,
            caged: false,
            radar_alt: false,
            brightness: 0.85,
        }
    }
}
/// CAS from compressible pitot pressure, with a Rayleigh pitot relation above M1.
/// Standard sea-level calibration; no instrument error model is claimed.
pub fn calibrated_speed(mach: f64, pressure: f64) -> f64 {
    let m = mach.max(0.);
    let ratio = if m <= 1. {
        (1. + 0.2 * m * m).powf(3.5)
    } else {
        (1.2 * m * m).powf(3.5) * (2.4 / (2.8 * m * m - 0.4)).powf(2.5)
    };
    let qc = pressure.max(0.) * (ratio - 1.);
    let sea_q = |v: f64| {
        let m = v / 340.294;
        let r = if m <= 1. {
            (1. + 0.2 * m * m).powf(3.5)
        } else {
            (1.2 * m * m).powf(3.5) * (2.4 / (2.8 * m * m - 0.4)).powf(2.5)
        };
        101325. * (r - 1.)
    };
    let (mut lo, mut hi) = (0., 2000.);
    for _ in 0..40 {
        let mid = (lo + hi) * 0.5;
        if sea_q(mid) < qc { lo = mid } else { hi = mid }
    }
    (lo + hi) * 0.5
}
pub fn project(f: &Flight, direction: DVec3) -> Option<Vec2> {
    if !direction.is_finite() {
        return None;
    }
    crate::cockpit::hud_projection(f.vehicle, (f.orientation.conjugate() * direction).as_vec3())
        .map(|p| p * 2.)
}
fn ladder_rung(f: &Flight, pitch: f64, caged: bool) -> Option<(Vec2, Vec2)> {
    let up = f.up();
    let track = if caged || f.velocity.length() < 5. {
        f.forward()
    } else {
        f.velocity.normalize()
    };
    let h = (track - up * track.dot(up)).normalize_or_zero();
    let right = h.cross(up).normalize_or_zero();
    let a = pitch.to_radians();
    let d = h * a.cos() + up * a.sin();
    Some((
        project(f, d - right * 0.085)?,
        project(f, d + right * 0.085)?,
    ))
}
pub fn aoa_bracket_offset(alpha: f64) -> f32 {
    ((alpha.to_degrees() - 8.1) * 8.).clamp(-60., 60.) as f32
}
fn in_field(p: Vec2) -> bool {
    p.is_finite() && p.x > 105. && p.x < 407. && p.y > 60. && p.y < 302.
}
struct Canvas<'a> {
    data: &'a mut [u8],
    gain: f32,
}
impl Canvas<'_> {
    // Signed-distance strokes at 2x logical resolution keep lines smooth and thin.
    fn line(&mut self, a: Vec2, b: Vec2) {
        if !a.is_finite() || !b.is_finite() {
            return;
        }
        let a = a * 2.;
        let b = b * 2.;
        let ab = b - a;
        let min = (a.min(b) - Vec2::splat(2.)).max(Vec2::ZERO).as_ivec2();
        let max = (a.max(b) + Vec2::splat(2.))
            .min(Vec2::new(WIDTH as f32 - 1., HEIGHT as f32 - 1.))
            .as_ivec2();
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let t = ((p - a).dot(ab) / ab.length_squared().max(0.01)).clamp(0., 1.);
                let alpha =
                    ((1.65 - p.distance(a + ab * t)).clamp(0., 1.) * 255. * self.gain) as u8;
                if alpha > 0 {
                    let i = (y as usize * WIDTH + x as usize) * 4;
                    self.data[i..i + 3].copy_from_slice(&[8, 255, 24]);
                    self.data[i + 3] = self.data[i + 3].max(alpha);
                }
            }
        }
    }
    fn text(&mut self, s: &str, x: f32, y: f32, scale: f32) {
        for (n, c) in s.chars().enumerate() {
            let glyph = super::glyph(c);
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        let p = Vec2::new(
                            x + (n as f32 * 6. + col as f32) * scale,
                            y + row as f32 * scale,
                        );
                        self.line(p, p);
                        if col < 4 && bits & (1 << (3 - col)) != 0 {
                            self.line(p, p + Vec2::X * scale);
                        }
                        if row < 6 && glyph[row + 1] & (1 << (4 - col)) != 0 {
                            self.line(p, p + Vec2::Y * scale);
                        }
                    }
                }
            }
        }
    }
    fn circle(&mut self, p: Vec2, r: f32) {
        for i in 0..48 {
            let a = i as f32 * std::f32::consts::TAU / 48.;
            let b = (i + 1) as f32 * std::f32::consts::TAU / 48.;
            self.line(
                p + Vec2::new(a.cos(), a.sin()) * r,
                p + Vec2::new(b.cos(), b.sin()) * r,
            );
        }
    }
    fn box_at(&mut self, p: Vec2, size: Vec2) {
        let a = p;
        let b = p + Vec2::X * size.x;
        let c = p + size;
        let d = p + Vec2::Y * size.y;
        for (a, b) in [(a, b), (b, c), (c, d), (d, a)] {
            self.line(a, b);
        }
    }
    fn diamond(&mut self, p: Vec2, r: f32) {
        for (a, b) in [
            (Vec2::NEG_Y, Vec2::X),
            (Vec2::X, Vec2::Y),
            (Vec2::Y, Vec2::NEG_X),
            (Vec2::NEG_X, Vec2::NEG_Y),
        ] {
            self.line(p + a * r, p + b * r);
        }
    }
    fn fpm(&mut self, p: Vec2, ghost: bool) {
        self.circle(p, 5.5);
        for side in [-1., 1.] {
            self.line(p + Vec2::X * side * 5.5, p + Vec2::X * side * 18.);
        }
        self.line(p - Vec2::Y * 5.5, p - Vec2::Y * 14.);
        if ghost {
            self.line(p - Vec2::splat(5.), p + Vec2::splat(5.));
        }
    }
}
/// Same inherited velocity, gravity, muzzle and lifetime as the current game gun.
/// Terrain intersection is numerical; no unsupported bomb delivery solution.
pub fn gun_impact(f: &Flight) -> Option<(DVec3, f64)> {
    let mut p = f.position + f.orientation * crate::aircraft::gun_origin(f.aircraft);
    let mut velocity = f.velocity + f.forward() * crate::combat::CANNON_MUZZLE_SPEED;
    for i in 0..250 {
        let old = p;
        velocity -= p.normalize() * 9.81 * 0.01;
        p += velocity * 0.01;
        if p.length() - RADIUS <= terrain_height(p.x, p.z).max(0.) {
            let (mut a, mut b) = (old, p);
            for _ in 0..16 {
                let m = (a + b) * 0.5;
                if m.length() - RADIUS > terrain_height(m.x, m.z).max(0.) {
                    a = m
                } else {
                    b = m
                }
            }
            return Some(((a + b) * 0.5, (i as f64 + 1.) * 0.01));
        }
    }
    None
}
pub fn draw(data: &mut [u8], f: &Flight, combat: &Combat) {
    assert_eq!(data.len(), WIDTH * HEIGHT * 4);
    data.fill(0);
    if !f.battery || f.damage.avionics < 0.1 || f.crashed {
        return;
    }
    let state = &f.hornet.hud;
    let mut c = Canvas {
        data,
        gain: state.brightness.clamp(0.1, 1.),
    };
    let rej = state.reject;
    // Heading tape wraps cleanly at North. Heading is TRUE in this Earth model.
    let heading = f.heading();
    for tick in -4..=4 {
        let h = (heading / 5.).round() * 5. + tick as f64 * 5.;
        let x = 256. + (h - heading) as f32 * 5.;
        c.line(
            Vec2::new(x, 30.),
            Vec2::new(x, if (h as i32) % 10 == 0 { 40. } else { 35. }),
        );
        if (h as i32) % 10 == 0 {
            c.text(
                &format!("{:02}", (h.rem_euclid(360.) / 10.).round() as i32),
                x - 9.,
                14.,
                1.5,
            );
        }
    }
    c.line(Vec2::new(250., 46.), Vec2::new(256., 40.));
    c.line(Vec2::new(256., 40.), Vec2::new(262., 46.));
    c.text("T", 373., 40., 1.2);
    let cas = calibrated_speed(f.mach, f.pressure) * 1.943844;
    let radalt =
        (f.position.length() - RADIUS - crate::carrier::ground(f, f.position)).max(0.) * 3.28084;
    let valid_radar = radalt < 5000.
        && f.roll().abs() < 60f64.to_radians()
        && f.pitch().abs() < 45f64.to_radians();
    c.text(&format!("{:>3.0}", cas), 20., 132., 2.1);
    let alt = if state.radar_alt && valid_radar {
        radalt
    } else {
        f.altitude() * 3.28084
    };
    c.text(&format!("{:>5.0}", alt), 415., 132., 2.0);
    c.text(
        if state.radar_alt && valid_radar {
            "R"
        } else {
            "B"
        },
        486.,
        135.,
        1.4,
    );
    if rej == 0 {
        c.box_at(Vec2::new(14., 126.), Vec2::new(55., 25.));
        c.box_at(Vec2::new(408., 126.), Vec2::new(89., 25.));
    }
    if rej < 2 {
        c.text(&format!("M {:.2}", f.mach), 20., 177., 1.6);
        c.text(&format!("G {:+.1}", f.g_load), 20., 198., 1.6);
        c.text(&format!("A {:.1}", f.alpha.to_degrees()), 20., 219., 1.6);
        c.text(
            &format!("{:+.0}", f.vertical_speed() * 196.8504),
            419.,
            177.,
            1.6,
        );
        c.text("FT/MIN", 419., 195., 1.1);
        c.text(
            &if valid_radar {
                format!("R {:.0}", radalt)
            } else {
                "R XXXX".into()
            },
            419.,
            219.,
            1.5,
        );
    }
    // Real attitude directions projected onto the actual combiner, not a screen-space horizon.
    for pitch in (-85i32..=85).step_by(if rej == 2 { 10 } else { 5 }) {
        if let Some((a, b)) = ladder_rung(f, pitch as f64, state.caged) {
            if [a, b].iter().all(|p| in_field(*p)) {
                let mid = (a + b) * 0.5;
                let gap = 0.23;
                for (start, end) in [(a, a.lerp(mid, 1. - gap)), (b, b.lerp(mid, 1. - gap))] {
                    if pitch < 0 {
                        for i in 0..3 {
                            c.line(
                                start.lerp(end, i as f32 / 3.),
                                start.lerp(end, (i as f32 + 0.55) / 3.),
                            );
                        }
                    } else {
                        c.line(start, end);
                    }
                    let tick = Vec2::new((b - a).y, -(b - a).x).normalize_or_zero()
                        * if pitch < 0 { -4. } else { 4. };
                    c.line(start, start + tick);
                }
                if pitch != 0 {
                    c.text(&pitch.to_string(), a.x - 19., a.y - 5., 1.2);
                    c.text(&pitch.to_string(), b.x + 5., b.y - 5., 1.2);
                }
            }
        }
    }
    // Waterline W is the aircraft reference, independent of velocity.
    if let Some(p) = project(f, f.forward()) {
        for s in [-1., 1.] {
            c.line(p + Vec2::new(s * 15., 0.), p + Vec2::new(s * 7., 5.));
            c.line(p + Vec2::new(s * 7., 5.), p);
        }
    }
    let fpm = if f.velocity.length() > 5. {
        project(f, f.velocity)
    } else {
        None
    };
    if let Some(actual) = fpm {
        let mut p = actual.clamp(Vec2::new(122., 75.), Vec2::new(390., 282.));
        let limited = p.distance(actual) > 1.;
        if state.caged {
            p.x = 256.;
            if actual.distance(p) > 8. && in_field(actual) {
                c.fpm(actual, true);
            }
        }
        if !limited || (f.elapsed * 4.).floor() as i64 % 2 == 0 {
            c.fpm(p, false);
            if limited {
                c.line(p - Vec2::splat(5.), p + Vec2::splat(5.));
            }
        }
        // Energy caret responds to specific longitudinal acceleration (estimate).
        let energy = (f.longitudinal_g * 18.).clamp(-20., 20.) as f32;
        let e = p + Vec2::new(25., -energy);
        c.line(e + Vec2::new(5., -4.), e);
        c.line(e, e + Vec2::new(5., 4.));
        if f.gear_extension > 0.98 && !f.landed && state.mode != MasterMode::Air {
            let a = p + Vec2::new(-29., aoa_bracket_offset(f.alpha));
            c.line(a - Vec2::Y * 10., a + Vec2::Y * 10.);
            for y in [-10., 0., 10.] {
                c.line(a + Vec2::Y * y, a + Vec2::new(7., y));
            }
            // A labelled incidence readout; this is not a separate physical AoA indexer.
            c.text(
                if f.alpha.to_degrees() > 8.8 {
                    "SLOW"
                } else if f.alpha.to_degrees() < 7.4 {
                    "FAST"
                } else {
                    "ON SPD"
                },
                16.,
                270.,
                1.3,
            );
        }
    }
    if state.mode == MasterMode::Nav {
        // Landing bank scale; limited pointer flashes beyond the scale.
        if rej == 0 {
            let centre = Vec2::new(256., 248.);
            for deg in [-45f32, -30., -15., -5., 0., 5., 15., 30., 45.] {
                let a = deg.to_radians();
                let d = Vec2::new(a.sin(), a.cos());
                c.line(centre + d * 63., centre + d * 69.);
            }
            let bank = f.roll().to_degrees() as f32;
            if bank.abs() < 47. || (f.elapsed * 4.).floor() as i64 % 2 == 0 {
                let d = Vec2::new(
                    bank.clamp(-47., 47.).to_radians().sin(),
                    bank.clamp(-47., 47.).to_radians().cos(),
                );
                let p = centre + d * 58.;
                c.line(p, p + Vec2::new(-4., -6.));
                c.line(p, p + Vec2::new(4., -6.));
            }
        }
        let dest = if f.carrier.active {
            crate::carrier::world(
                DVec3::new(crate::carrier::lane(88.), crate::carrier::DECK, 88.),
                f.elapsed,
            )
        } else {
            surface_point(0., 0., RUNWAY_HEIGHT)
        };
        let delta = dest - f.position;
        let range = delta.length() / 1852.;
        let (north, east) = geographic_axes(f.up());
        let bearing = delta
            .dot(east)
            .atan2(delta.dot(north))
            .to_degrees()
            .rem_euclid(360.);
        c.text(
            &format!(
                "{} {:05.1}",
                if f.carrier.active { "CVN" } else { "HOME" },
                range
            ),
            359.,
            333.,
            1.4,
        );
        c.text(&format!("{:03.0} T", bearing), 411., 351., 1.4);
        if let Some(p) = project(f, delta).filter(|p| in_field(*p)) {
            c.diamond(p, 7.);
        }
        let error = (bearing - heading + 180.).rem_euclid(360.) - 180.;
        let x = 256. + error.clamp(-20., 20.) as f32 * 5.;
        c.box_at(Vec2::new(x - 3., 46.), Vec2::splat(6.));
    }
    if state.mode == MasterMode::Air {
        air(&mut c, f, combat);
    }
    if state.mode == MasterMode::Ground {
        ground(&mut c, f, combat);
    }
    c.text(state.mode.label(), 20., 333., 1.8);
    c.text(&format!("FUEL {:04.0}", (f.fuel+crate::aircraft::hornet::stores::fuel(f)) * 2.204623), 20., 353., 1.25);
    if state.caged {
        c.text("CAGED", 220., 354., 1.2);
    }
    let caution = if f.fuel+crate::aircraft::hornet::stores::fuel(f) < 300. {
        "FUEL"
    } else if f.gear_extension < 0.98 && radalt < 200. && f.vertical_speed() < -1. {
        "CHECK GEAR"
    } else if f.carrier.active && f.gear_extension > 0.98 && !f.systems.hook_down {
        "HOOK"
    } else if f.hornet.fbw.mode == crate::aircraft::hornet::fbw::Mode::Failed {
        "FCS FAIL"
    } else {
        ""
    };
    if (f.elapsed * 2.).floor() as i64 % 2 == 0 {
        c.text(caution, 213., 65., 1.5);
    }
}
fn air(c: &mut Canvas, f: &Flight, combat: &Combat) {
    let heat = combat.weapon == Weapon::Heat;
    c.text(if heat { "9X" } else { "120" }, 20., 297., 1.6);
    c.text(
        &format!(
            "{} {}",
            combat.ammo[combat.weapon.index()],
            if combat.active { "ARM" } else { "SAFE" }
        ),
        20.,
        315.,
        1.2,
    );
    let bore = project(f, f.forward()).unwrap_or(Vec2::new(256., 192.));
    c.circle(bore, if heat { 45. } else { 66. });
    if !combat.active {
        return;
    }
    if let Some(t) = combat.targets.get(combat.selected).filter(|t| t.alive) {
        let range = t.position.distance(f.position);
        let closure = crate::combat::hud::closing(f, t);
        let acquired = combat.cue(f).is_ok();
        let locked = acquired && combat.lock >= combat.weapon.lock_time();
        if let Some(p) = project(f, t.position - f.position).filter(|p| in_field(*p)) {
            c.box_at(p - Vec2::splat(9.), Vec2::splat(18.));
            if locked {
                c.diamond(p, 15.);
            }
            if let Some(gun) = crate::combat::hud::gun_solution(f, t) {
                if let Some(g) = project(f, gun.direction).filter(|p| in_field(*p)) {
                    c.circle(g, 8.);
                    c.line(g - Vec2::X * 3., g + Vec2::X * 3.);
                }
            }
        } else {
            let body = f.orientation.conjugate() * (t.position - f.position);
            let angle = body.x.atan2(-body.z) as f32;
            let d = Vec2::new(angle.sin(), -angle.cos());
            let p = Vec2::new(256., 185.) + d * 90.;
            c.line(p, p - d * 15. + Vec2::new(-d.y, d.x) * 5.);
            c.line(p, p - d * 15. - Vec2::new(-d.y, d.x) * 5.);
            c.text("LOS", p.x + 8., p.y, 1.1);
        }
        c.text(&format!("R {:.1}", range / 1852.), 418., 270., 1.4);
        c.text(&format!("VC {:+.0}", closure * 1.943844), 406., 290., 1.2);
        c.text(
            if locked {
                "LOCK"
            } else if acquired {
                "ACQ"
            } else {
                "NO LOCK"
            },
            220.,
            320.,
            1.5,
        );
        // Readiness from the existing weapon logic. No invented NEZ / PK scale.
        c.text(
            if locked && combat.ammo[combat.weapon.index()] > 0 {
                "RDY"
            } else {
                "INHIBIT"
            },
            419.,
            311.,
            1.2,
        );
    }
}
fn ground(c: &mut Canvas, f: &Flight, combat: &Combat) {
    c.text("GUN CCIP", 20., 296., 1.4);
    c.text(
        if combat.active && combat.fight.is_some() {
            "ARM"
        } else {
            "SAFE"
        },
        20.,
        315.,
        1.2,
    );
    if let Some((impact, time)) = gun_impact(f) {
        if let Some(p) = project(f, impact - f.position).filter(|p| in_field(*p)) {
            c.circle(p, 9.);
            c.line(p - Vec2::X * 3., p + Vec2::X * 3.);
            if let Some(bore) = project(f, f.forward()) {
                let end = bore.clamp(Vec2::new(110., 65.), Vec2::new(402., 295.));
                c.line(end, p - Vec2::Y * 10.);
            }
            c.text(
                &format!("{:.1} RNG", impact.distance(f.position) / 1852.),
                403.,
                270.,
                1.3,
            );
            c.text(&format!("{:.2} SEC", time), 403., 290., 1.3);
        } else {
            c.text("CCIP LIMIT", 210., 300., 1.2);
        }
    } else {
        c.text("NO SOLUTION", 205., 300., 1.2);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn flight() -> Flight {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Hornet);
        f
    }
    #[test]
    fn compressible_cas_calibration_and_supersonic_continuity() {
        for m in [0., 0.2, 0.8, 1., 1.5, 2.] {
            assert!((calibrated_speed(m, 101325.) - m * 340.294).abs() < 1e-6);
        }
        assert!(calibrated_speed(0.8, 30000.) < calibrated_speed(0.8, 101325.));
        assert!(
            (calibrated_speed(1. + 1e-7, 50000.) - calibrated_speed(1. - 1e-7, 50000.)).abs()
                < 0.001
        );
    }
    #[test]
    fn projection_and_landing_bracket_signs() {
        let f = flight();
        let centre = project(&f, f.forward()).unwrap();
        assert!(
            project(&f, f.forward() + f.orientation * DVec3::X * 0.05)
                .unwrap()
                .x
                > centre.x
        );
        assert!(
            project(&f, f.forward() + f.orientation * DVec3::Y * 0.05)
                .unwrap()
                .y
                < centre.y
        );
        assert!(project(&f, -f.forward()).is_none());
        assert!(aoa_bracket_offset(9f64.to_radians()) > 0.);
        assert!(aoa_bracket_offset(7f64.to_radians()) < 0.);
        assert!(aoa_bracket_offset(8.1f64.to_radians()).abs() < 1e-5);
    }
    #[test]
    fn raster_modes_power_declutter_and_physics_boundaries() {
        let mut f = flight();
        let combat = Combat::default();
        let mut data = vec![0; WIDTH * HEIGHT * 4];
        let pose = (
            f.position,
            f.velocity,
            f.orientation,
            f.fuel,
            f.flaps,
            f.throttle,
        );
        let mut results = Vec::new();
        for mode in [MasterMode::Nav, MasterMode::Air, MasterMode::Ground] {
            f.hornet.hud.mode = mode;
            draw(&mut data, &f, &combat);
            assert!(data.chunks_exact(4).filter(|p| p[3] > 0).count() > 4000);
            results.push(data.clone());
        }
        assert_ne!(results[0], results[1]);
        assert_ne!(results[1], results[2]);
        f.hornet.hud.mode = MasterMode::Nav;
        f.hornet.hud.reject = 2;
        draw(&mut data, &f, &combat);
        assert!(
            data.chunks_exact(4).filter(|p| p[3] > 0).count()
                < results[0].chunks_exact(4).filter(|p| p[3] > 0).count()
        );
        f.battery = false;
        draw(&mut data, &f, &combat);
        assert!(data.iter().all(|v| *v == 0));
        f.battery = true;
        f.damage.avionics = 0.;
        draw(&mut data, &f, &combat);
        assert!(data.iter().all(|v| *v == 0));
        assert_eq!(
            pose,
            (
                f.position,
                f.velocity,
                f.orientation,
                f.fuel,
                f.flaps,
                f.throttle
            )
        );
        assert_eq!(MasterMode::Nav.next().next().next(), MasterMode::Nav);
    }
    #[test]
    fn terrain_gun_prediction_intersects_surface_and_rejects_sky() {
        let mut f = flight();
        f.position = surface_point(0., 0., 200.);
        f.orientation = bevy::math::DQuat::from_rotation_x(-0.15);
        f.velocity = f.forward() * 150.;
        let (p, t) = gun_impact(&f).unwrap();
        assert!((p.length() - RADIUS - terrain_height(p.x, p.z).max(0.)).abs() < 0.002);
        assert!(t > 0. && t < 2.5);
        f.orientation = bevy::math::DQuat::from_rotation_x(0.5);
        f.velocity = f.forward() * 150.;
        assert!(gun_impact(&f).is_none());
    }
    #[test]
    fn banked_crosswind_horizon_stays_conformal() {
        let mut f = flight();
        f.orientation *= bevy::math::DQuat::from_rotation_z(0.4);
        f.velocity += f.orientation * DVec3::X * 20.;
        for caged in [false, true] {
            let (a, b) = ladder_rung(&f, 0., caged).unwrap();
            // Invert the combiner intersection back into body-space rays.
            for p in [a, b] {
                let local = Vec3::new(
                    (p.x / 2. - 128.) / 256. * 0.30,
                    (96. - p.y / 2.) / 192. * 0.18,
                    0.,
                );
                let point = Quat::from_rotation_x(-0.14) * local + Vec3::new(0., 1.05, -3.05);
                let ray = f.orientation
                    * (point - crate::cockpit::eye(f.vehicle))
                        .as_dvec3()
                        .normalize();
                assert!(
                    ray.dot(f.up()).abs() < 1e-6,
                    "horizon must remain at zero world elevation"
                );
            }
        }
    }
    #[test]
    fn render_cost_sample() {
        let f = flight();
        let combat = Combat::default();
        let mut data = vec![0; WIDTH * HEIGHT * 4];
        let mut samples = Vec::new();
        for _ in 0..120 {
            let now = std::time::Instant::now();
            draw(&mut data, &f, &combat);
            std::hint::black_box(&data);
            samples.push(now.elapsed().as_secs_f64() * 1000.);
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "HUD CPU raster mean_ms={:.3} p95_ms={:.3}, 1024x768, GPU/upload excluded",
            samples.iter().sum::<f64>() / samples.len() as f64,
            samples[samples.len() * 95 / 100]
        );
    }
    #[test]
    fn export_review_frames() {
        use bevy::{
            asset::RenderAssetUsages,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat},
        };
        let out = std::path::Path::new("tests/hornet-hud");
        std::fs::create_dir_all(out).unwrap();
        let mut f = flight();
        let mut combat = Combat::default();
        for name in [
            "nav", "air", "ground", "landing", "bank", "caged", "reject", "off",
        ] {
            f.hornet.hud = State::default();
            f.battery = true;
            if name == "landing" {
                f = crate::carrier::scenario(false);
            }
            if name == "air" {
                f.hornet.hud.mode = MasterMode::Air;
                combat.start(&f, 1);
                combat.targets[0].position = f.position + f.forward() * 1600.;
                combat.targets[0].velocity = f.velocity;
                combat.lock = 2.;
            }
            if name == "ground" {
                f.hornet.hud.mode = MasterMode::Ground;
            }
            if name == "bank" {
                f.orientation *= bevy::math::DQuat::from_rotation_z(30f64.to_radians());
            }
            if name == "caged" {
                f.hornet.hud.caged = true;
                f.velocity += f.orientation * DVec3::X * 12.;
            }
            if name == "reject" {
                f.hornet.hud.reject = 2;
            }
            if name == "off" {
                f.battery = false;
            }
            let mut image = Image::new_fill(
                Extent3d {
                    width: WIDTH as u32,
                    height: HEIGHT as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[0, 0, 0, 0],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            );
            draw(image.data.as_mut().unwrap(), &f, &combat);
            image
                .try_into_dynamic()
                .unwrap()
                .save(out.join(format!("{name}.png")))
                .unwrap();
        }
    }
}
