//! Bounded, simulation-time terrain preview. Guidance never changes aircraft state.
use super::*;
#[derive(Clone, Copy, Default, Debug)]
pub(super) struct Advice {
    pub active: bool,
    pub path: f64,
    pub bank: f64,
}
#[derive(Clone, Default, Debug)]
pub struct Guard {
    at: f64,
    position: DVec3,
    velocity: DVec3,
    goal: DVec3,
    valid: bool,
    hold_until: f64,
    advice: Advice,
    pub scans: u64,
}
impl Guard {
    pub(super) fn update(&mut self, f: &Flight, desired: DVec3) -> Advice {
        let changed = self.goal.dot(desired) < 0.966;
        if self.valid
            && (f.elapsed < self.at || self.position.distance_squared(f.position) > 400. * 400.)
        {
            self.hold_until = f.elapsed;
        }
        if !self.valid
            || f.elapsed < self.at
            || f.elapsed - self.at >= 0.125
            || self.position.distance_squared(f.position) > 400. * 400.
            || (changed && f.elapsed - self.at >= 0.0625)
        {
            let mut advice = preview(f, desired, contact_height_at);
            if advice.active {
                self.hold_until = f.elapsed + 0.8;
            } else if f.elapsed < self.hold_until {
                advice.active = true;
                advice.path = advice.path.max(0.10);
            }
            self.advice = advice;
            self.at = f.elapsed;
            self.position = f.position;
            self.velocity = f.velocity;
            self.goal = desired;
            self.valid = true;
            self.scans += 1;
        }
        self.advice
    }
}
fn trace(
    f: &Flight,
    heading: DVec3,
    turn: f64,
    margin: f64,
    ground: &impl Fn(DVec3) -> f64,
) -> (f64, f64) {
    let up = f.up();
    let speed = f.velocity.length().max(100.);
    let vertical = f.velocity.dot(up);
    let horizon = (12. + speed / 60.).clamp(14., 22.);
    let step = horizon / 32.;
    let rate = ((crate::environment::mu() / f.position.length_squared()) * 0.7_f64.tan() / speed)
        .min(0.11);
    let mut position = f.position;
    let altitude = f.altitude();
    let mut deficit = f64::NEG_INFINITY;
    let mut path = -0.25_f64;
    for i in 0..=32 {
        let time = i as f64 * step;
        if i > 0 {
            // Allow for roll response before crediting an escape turn.
            let next = (turn.signum() * rate * (time - 1.5).max(0.)).clamp(-turn.abs(), turn.abs());
            let dir = DQuat::from_axis_angle(up, next) * heading;
            position += dir * speed * step;
        }
        let floor = ground(position) + margin;
        let distance = (speed * time).max(100.);
        // Include descent and the distance spent rolling upright before pull-up.
        let delay = 1.0 + f.roll().abs() / 1.2;
        let loss = (-vertical).max(0.) * delay;
        path = path.max((floor + loss - altitude).atan2(distance));
        deficit = deficit.max(floor - (altitude + vertical * time));
    }
    (deficit, path)
}
pub(super) fn preview(f: &Flight, desired: DVec3, ground: impl Fn(DVec3) -> f64) -> Advice {
    let up = f.up();
    let speed = f.velocity.length();
    let heading = (f.velocity - up * f.velocity.dot(up)).normalize_or(f.forward());
    let right = heading.cross(up).normalize_or(DVec3::X);
    let intended = desired
        .dot(right)
        .atan2(desired.dot(heading))
        .clamp(-0.85, 0.85);
    let margin = (130. + speed * 0.35).clamp(170., 340.);
    let straight = trace(f, heading, 0., margin, &ground);
    let planned = if intended.abs() > 0.15 {
        trace(f, heading, -intended, margin, &ground)
    } else {
        straight
    };
    if straight.0 <= 0. && planned.0 <= 0. {
        return Advice::default();
    }
    let mut bank = 0.;
    let mut path = straight.1.max(planned.1);
    // Side corridors are evaluated only when a significant obstacle is present.
    if path > 0.30 {
        let left = trace(f, heading, 0.75, margin, &ground);
        let right_path = trace(f, heading, -0.75, margin, &ground);
        let (best, side) = if left.1 < right_path.1 {
            (left, -1.)
        } else {
            (right_path, 1.)
        };
        if best.1 + 0.10 < straight.1 {
            path = best.1;
            bank = side * 0.45;
        }
    }
    Advice {
        active: true,
        path: (path + 0.10).clamp(0.16, 0.65),
        bank,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn aircraft() -> Flight {
        let mut f = airframe();
        f.position = crate::terrain::point(0., 0., 1400.);
        f.orientation = super::super::super::facing(-DVec3::Z, f.up());
        f.velocity = f.forward() * 250.;
        f
    }
    #[test]
    fn restarting_or_teleporting_discards_the_previous_recovery_hold() {
        let mut g = Guard::default();
        let mut f = aircraft();
        f.position = crate::terrain::point(0., 0., 20.);
        f.elapsed = 10.;
        assert!(g.update(&f, f.forward()).active);
        f.position = crate::terrain::point(0., 0., 5000.);
        f.elapsed = 0.;
        assert!(!g.update(&f, f.forward()).active);
    }
    #[test]
    fn ridge_between_current_position_and_old_five_second_probe_is_detected() {
        let f = aircraft();
        let a = preview(&f, f.forward(), |p| {
            if p.z < -500. && p.z > -1000. {
                1500.
            } else {
                0.
            }
        });
        assert!(a.active && a.path > 0.3);
        assert_eq!(f.position, aircraft().position);
    }
    #[test]
    fn intended_turn_into_a_ridge_is_checked_and_clear_high_flight_is_not_overridden() {
        let f = aircraft();
        assert!(!preview(&f, f.forward(), |_| 0.).active);
        let a = preview(&f, (f.forward() + DVec3::X).normalize(), |p| {
            if p.x > 250. { 2400. } else { 0. }
        });
        assert!(a.active);
    }
    #[test]
    fn fast_descent_and_roll_need_earlier_recovery() {
        let mut f = aircraft();
        f.velocity -= f.up() * 110.;
        assert!(preview(&f, f.forward(), |_| 0.).active);
    }
    #[test]
    fn scan_cadence_is_bounded_and_teleport_invalidates_advice() {
        let mut g = Guard::default();
        let mut f = aircraft();
        for i in 0..120 {
            f.elapsed = i as f64 / 120.;
            g.update(&f, f.forward());
        }
        assert!(g.scans <= 9);
        let scans = g.scans;
        f.position += DVec3::X * 1000.;
        g.update(&f, f.forward());
        assert_eq!(g.scans, scans + 1);
    }
}
