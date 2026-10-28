//! Signed accelerometer-driven motion, independent of event/emotion clips.
//! +X accelerates aircraft right; +forward accelerates along body -Z. The
//! occupant and loose ears lean opposite that acceleration, never camera axes.
use super::animation::{Emotion, Pose};
use bevy::prelude::*;
#[derive(Clone, Copy, Debug)]
pub struct Load {
    pub vertical: f32,
    pub lateral: f32,
    pub forward: f32,
    pub airborne: bool,
}
impl Default for Load {
    fn default() -> Self {
        Self {
            vertical: 1.,
            lateral: 0.,
            forward: 0.,
            airborne: false,
        }
    }
}
fn finite(v: f32, fallback: f32) -> f32 {
    if v.is_finite() {
        v.clamp(-20., 20.)
    } else {
        fallback
    }
}
impl Load {
    pub fn new(vertical: f64, lateral: f64, forward: f64, airborne: bool) -> Self {
        Self {
            vertical: finite(vertical as f32, 1.),
            lateral: finite(lateral as f32, 0.),
            forward: finite(forward as f32, 0.),
            airborne,
        }
    }
    pub fn positive(self) -> f32 {
        ramp(self.vertical, &[(1.5, 0.), (3., 0.2), (5., 0.6), (9., 1.)])
    }
    pub fn negative(self) -> f32 {
        ramp(
            -self.vertical,
            &[(0., 0.), (0.5, 0.2), (1.5, 0.6), (3., 1.)],
        )
    }
    pub fn side(self) -> f32 {
        ramp(
            self.lateral.abs(),
            &[(0.15, 0.), (0.5, 0.2), (1.2, 0.55), (2.5, 1.)],
        ) * self.lateral.signum()
    }
    pub fn longitudinal(self) -> f32 {
        ramp(
            self.forward.abs(),
            &[(0.08, 0.), (0.5, 0.3), (1., 0.6), (2., 1.)],
        ) * self.forward.signum()
    }
    pub fn floating(self) -> bool {
        self.airborne
            && self.vertical.abs() < 0.25
            && self.lateral.abs() < 0.2
            && self.forward.abs() < 0.2
    }
    pub fn strength(self) -> f32 {
        self.positive()
            .max(self.negative())
            .max(self.side().abs())
            .max(self.longitudinal().abs())
    }
    pub fn emotion(self) -> Option<Emotion> {
        let v = self.positive().max(self.negative());
        let s = self.side().abs();
        let f = self.longitudinal().abs();
        if v.max(s).max(f) < 0.06 {
            return None;
        }
        Some(if v >= s && v >= f {
            if self.vertical < 0. {
                Emotion::NegativeG
            } else if self.vertical > 7. {
                Emotion::Bracing
            } else {
                Emotion::PositiveG
            }
        } else if s >= f {
            Emotion::SideG
        } else if self.forward > 0. {
            Emotion::Focused
        } else {
            Emotion::Bracing
        })
    }
    pub fn label(self) -> Option<&'static str> {
        let v = self.positive().max(self.negative());
        let s = self.side().abs();
        let f = self.longitudinal().abs();
        if self.floating() {
            return Some("FLOATING");
        }
        if v.max(s).max(f) < 0.06 {
            return None;
        }
        Some(if v >= s && v >= f {
            if self.vertical >= 0. {
                if v < 0.4 {
                    "+G LIGHT"
                } else if v < 0.85 {
                    "+G STRONG"
                } else {
                    "+G EXTREME"
                }
            } else if v < 0.4 {
                "-G LIGHT"
            } else if v < 0.85 {
                "-G STRONG"
            } else {
                "-G EXTREME"
            }
        } else if s >= f {
            if self.lateral > 0. {
                "PULLED LEFT"
            } else {
                "PULLED RIGHT"
            }
        } else if self.forward > 0. {
            "PUSHED BACK"
        } else {
            "PULLED FORWARD"
        })
    }
}
fn ramp(x: f32, knots: &[(f32, f32)]) -> f32 {
    for p in knots.windows(2) {
        if x <= p[1].0 {
            return p[0].1 + (p[1].1 - p[0].1) * ((x - p[0].0) / (p[1].0 - p[0].0)).clamp(0., 1.);
        }
    }
    knots.last().unwrap().1
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    Float,
    PositiveLight,
    PositiveStrong,
    PositiveExtreme,
    NegativeLight,
    NegativeStrong,
    NegativeExtreme,
    LeftLight,
    LeftStrong,
    RightLight,
    RightStrong,
    ForwardLight,
    ForwardStrong,
    BackwardLight,
    BackwardStrong,
}
impl Preset {
    pub const ALL: [Self; 15] = [
        Self::Float,
        Self::PositiveLight,
        Self::PositiveStrong,
        Self::PositiveExtreme,
        Self::NegativeLight,
        Self::NegativeStrong,
        Self::NegativeExtreme,
        Self::LeftLight,
        Self::LeftStrong,
        Self::RightLight,
        Self::RightStrong,
        Self::ForwardLight,
        Self::ForwardStrong,
        Self::BackwardLight,
        Self::BackwardStrong,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Float => "ZERO G",
            Self::PositiveLight => "+2.5 G",
            Self::PositiveStrong => "+5 G",
            Self::PositiveExtreme => "+9 G",
            Self::NegativeLight => "−0.5 G",
            Self::NegativeStrong => "−1.5 G",
            Self::NegativeExtreme => "−3 G",
            Self::LeftLight => "LEFT 0.5 G",
            Self::LeftStrong => "LEFT 2 G",
            Self::RightLight => "RIGHT 0.5 G",
            Self::RightStrong => "RIGHT 2 G",
            Self::ForwardLight => "FORWARD 0.5 G",
            Self::ForwardStrong => "FORWARD 2 G",
            Self::BackwardLight => "BACKWARD 0.5 G",
            Self::BackwardStrong => "BACKWARD 2 G",
        }
    }
    pub fn slug(self) -> &'static str {
        match self {
            Self::Float => "g-zero",
            Self::PositiveLight => "g-positive-light",
            Self::PositiveStrong => "g-positive-strong",
            Self::PositiveExtreme => "g-positive-extreme",
            Self::NegativeLight => "g-negative-light",
            Self::NegativeStrong => "g-negative-strong",
            Self::NegativeExtreme => "g-negative-extreme",
            Self::LeftLight => "g-left-light",
            Self::LeftStrong => "g-left-strong",
            Self::RightLight => "g-right-light",
            Self::RightStrong => "g-right-strong",
            Self::ForwardLight => "g-forward-light",
            Self::ForwardStrong => "g-forward-strong",
            Self::BackwardLight => "g-backward-light",
            Self::BackwardStrong => "g-backward-strong",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.slug() == s)
    }
    pub fn load(self) -> Load {
        let (v, s, f) = match self {
            Self::Float => (0., 0., 0.),
            Self::PositiveLight => (2.5, 0., 0.),
            Self::PositiveStrong => (5., 0., 0.),
            Self::PositiveExtreme => (9., 0., 0.),
            Self::NegativeLight => (-0.5, 0., 0.),
            Self::NegativeStrong => (-1.5, 0., 0.),
            Self::NegativeExtreme => (-3., 0., 0.),
            Self::LeftLight => (1., -0.5, 0.),
            Self::LeftStrong => (1., -2., 0.),
            Self::RightLight => (1., 0.5, 0.),
            Self::RightStrong => (1., 2., 0.),
            Self::ForwardLight => (1., 0., 0.5),
            Self::ForwardStrong => (1., 0., 2.),
            Self::BackwardLight => (1., 0., -0.5),
            Self::BackwardStrong => (1., 0., -2.),
        };
        Load::new(v, s, f, true)
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Float => {
                "Near weightlessness: head lifts gently and relaxed ears float. Ground contact disables this reaction."
            }
            Self::PositiveLight => {
                "Light positive load: subtle compression, focused eyes and a firmer grip."
            }
            Self::PositiveStrong => {
                "Strong positive load: seat compression, narrowed eyes, tense breathing and pinned ears."
            }
            Self::PositiveExtreme => {
                "Extreme positive load: deep squint, braced shoulders and a small strain tremor."
            }
            Self::NegativeLight => {
                "Light negative load: uneasy brows, a lifting head and gently rising ears."
            }
            Self::NegativeStrong => {
                "Strong negative load: uncomfortable grimace, raised ears and a tighter harness brace."
            }
            Self::NegativeExtreme => {
                "Extreme negative load: pronounced grimace, floating ears and a restrained jaw tremble."
            }
            Self::LeftLight | Self::LeftStrong => {
                "Aircraft accelerates left: pilot leans right, with slower trailing ears. Stronger load increases the lean and brace."
            }
            Self::RightLight | Self::RightStrong => {
                "Aircraft accelerates right: pilot leans left, with slower trailing ears. Stronger load increases the lean and brace."
            }
            Self::ForwardLight | Self::ForwardStrong => {
                "Aircraft accelerates forward: pilot presses back into the seat, chin lifts and ears trail backward."
            }
            Self::BackwardLight | Self::BackwardStrong => {
                "Aircraft accelerates backward or brakes: pilot pitches forward against the harness and ears swing toward the muzzle."
            }
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Motion {
    pub head_offset: Vec3,
    pub chest: Vec3,
    pub pos: f32,
    pub neg: f32,
    pub side: f32,
    pub forward: f32,
    pub ear_side: f32,
    pub ear_forward: f32,
    pub ear_negative: f32,
    pub float: f32,
}
impl Motion {
    /// Long hanging ears float past horizontal, with a soft overshoot and
    /// unequal sway. Replace the ordinary negative-G hinge lift, retaining
    /// lateral/longitudinal motion and the emotional clip's ear pose.
    pub fn apply_floppy_ears(self, p: &mut Pose, time: f32) {
        let lift = self.ear_negative.max(0.);
        let flutter = |phase: f32| {
            lift * (0.13 * (time * 4.1 + phase).sin() + 0.045 * (time * 7.3 + phase * 1.7).sin())
        };
        let extra = self.ear_negative * 1.55 - self.neg * 0.95;
        p.ear_l += Vec3::new(
            lift * 0.12 * (time * 3.6 + 0.4).sin(),
            lift * 0.09 * (time * 2.5 + 0.8).sin(),
            -extra - flutter(0.),
        );
        p.ear_r += Vec3::new(
            lift * 0.12 * (time * 3.6 + 1.1).sin(),
            -lift * 0.09 * (time * 2.5 + 1.4).sin(),
            extra + flutter(0.85),
        );
    }
    pub fn apply(self, p: &mut Pose, time: f32) {
        let strain = self.pos.max(self.neg);
        let horizontal = self.side.abs().max(self.forward.abs());
        p.head += Vec3::new(
            self.pos * 0.04 - self.neg * 0.05
                + self.forward * if self.forward > 0. { 0.10 } else { 0.16 },
            0.,
            self.side * 0.15,
        );
        p.head.z += (time * 23.).sin() * self.pos.powi(3) * 0.004;
        p.head_y -= self.pos * 0.016;
        p.head_y += self.neg * 0.010 + self.float * 0.006;
        p.ear_l += Vec3::new(
            -self.ear_forward * 0.32,
            0.,
            -self.neg * 0.95 - self.ear_side * 0.28 - self.float * 0.22,
        );
        p.ear_r += Vec3::new(
            -self.ear_forward * 0.32,
            0.,
            self.neg * 0.95 - self.ear_side * 0.28 + self.float * 0.22,
        );
        p.gaze += Vec2::new(self.side * 0.16, -self.forward * 0.09);
        p.lids = (p.lids + Vec2::splat(self.pos * 0.36 + self.neg * 0.18 + horizontal * 0.10))
            .min(Vec2::splat(0.94));
        p.brow += Vec2::splat(-self.pos * 0.004 + self.neg * 0.004);
        p.brow_angle += Vec2::new(-1., 1.) * (self.pos * 0.12 - self.neg * 0.10);
        p.brace = p.brace.max(strain * 0.85 + horizontal * 0.75);
        p.jaw *= 1. - self.pos * 0.55;
        p.jaw += (time * 15.).sin().abs() * self.neg.powi(2) * 0.012;
        p.breath *= 1. - self.pos * 0.60;
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Track {
    body: Vec4,
    ears: Vec2,
    negative_lift: f32,
    negative_velocity: f32,
}
impl Default for Track {
    fn default() -> Self {
        Self {
            body: Vec4::new(1., 0., 0., 0.),
            ears: Vec2::ZERO,
            negative_lift: 0.,
            negative_velocity: 0.,
        }
    }
}
impl Track {
    pub fn update(&mut self, l: Load, dt: f32) -> Motion {
        let dt = dt.clamp(0., 0.1);
        let a = 1. - (-dt / 0.055).exp();
        let e = 1. - (-dt / 0.14).exp();
        self.body = self.body.lerp(
            Vec4::new(
                l.vertical,
                l.lateral,
                l.forward,
                if l.floating() { 1. } else { 0. },
            ),
            a,
        );
        self.ears = self.ears.lerp(Vec2::new(l.side(), l.longitudinal()), e);
        // Underdamped ear response, integrated in bounded substeps so a frame
        // hitch cannot produce a large flip. A release produces a short flop.
        let steps = (dt * 120.).ceil().max(1.) as usize;
        let h = dt / steps as f32;
        for _ in 0..steps {
            self.negative_velocity +=
                (144. * (l.negative() - self.negative_lift) - 15.6 * self.negative_velocity) * h;
            self.negative_lift =
                (self.negative_lift + self.negative_velocity * h).clamp(-0.12, 1.12);
        }
        let b = Load::new(
            self.body.x as f64,
            self.body.y as f64,
            self.body.z as f64,
            l.airborne,
        );
        let pos = b.positive();
        let neg = b.negative();
        let side = b.side();
        let forward = b.longitudinal();
        Motion {
            head_offset: Vec3::new(-side * 0.008, 0., forward * 0.012),
            chest: Vec3::new(forward * 0.055 - pos * 0.025 + neg * 0.02, 0., side * 0.045),
            pos,
            neg,
            side,
            forward,
            ear_side: self.ears.x,
            ear_forward: self.ears.y,
            ear_negative: self.negative_lift,
            float: self.body.w,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn settled(l: Load) -> Motion {
        let mut t = Track::default();
        let mut m = Motion::default();
        for _ in 0..60 {
            m = t.update(l, 1. / 60.);
        }
        m
    }
    #[test]
    fn graded_loads_are_continuous_and_monotonic() {
        let mut previous = 0.;
        for i in 150..=1000 {
            let l = Load::new(i as f64 / 100., 0., 0., true);
            assert!(l.positive() >= previous);
            previous = l.positive();
        }
        let mut previous = 0.;
        for i in 0..=350 {
            let l = Load::new(-(i as f64) / 100., 0., 0., true);
            assert!(l.negative() >= previous);
            previous = l.negative();
        }
        assert!(Preset::PositiveLight.load().positive() < Preset::PositiveStrong.load().positive());
        assert!(
            Preset::PositiveStrong.load().positive() < Preset::PositiveExtreme.load().positive()
        );
        assert!(Preset::NegativeLight.load().negative() < Preset::NegativeStrong.load().negative());
        assert!(
            Preset::NegativeStrong.load().negative() < Preset::NegativeExtreme.load().negative()
        );
    }
    #[test]
    fn lean_opposes_aircraft_acceleration_and_mirrors_both_directions() {
        let right = settled(Preset::RightStrong.load());
        let left = settled(Preset::LeftStrong.load());
        let mut a = Pose::default();
        let mut b = a;
        right.apply(&mut a, 0.);
        left.apply(&mut b, 0.);
        assert!(a.head.z > 0. && a.ear_l.z < 0. && right.head_offset.x < 0.);
        assert!((a.head.z + b.head.z).abs() < 1e-5);
        assert!((right.chest.z + left.chest.z).abs() < 1e-5);
        let forward = settled(Preset::ForwardStrong.load());
        let back = settled(Preset::BackwardStrong.load());
        assert!(forward.head_offset.z > 0. && forward.chest.x > 0. && forward.ear_forward > 0.);
        assert!((forward.head_offset.z + back.head_offset.z).abs() < 1e-5);
    }
    #[test]
    fn body_responds_quickly_ears_lag_and_direction_reverses_without_a_snap() {
        let mut t = Track::default();
        let load = Preset::RightStrong.load();
        let first = t.update(load, 1. / 60.);
        assert!(first.side > 0. && first.side < 0.6);
        let mut m = first;
        for _ in 0..5 {
            m = t.update(load, 1. / 60.);
        }
        assert!(m.side > 0.5 && m.ear_side < m.side);
        let reverse = t.update(Preset::LeftStrong.load(), 1. / 60.);
        assert!(reverse.side > -0.2);
        for _ in 0..20 {
            m = t.update(Preset::LeftStrong.load(), 1. / 60.);
        }
        assert!(m.side < -0.7);
    }
    #[test]
    fn combined_loads_remain_finite_and_do_not_erase_explosion_expression() {
        let mut m = settled(Load::new(9., -3., 2., true));
        let mut p = super::super::animation::sample(Emotion::Startled, 0.1, 0.);
        m.apply(&mut p, 0.1);
        assert!(p.jaw > 0.05 && p.brace > 0.8 && p.head.z < 0.);
        assert!(p.head.is_finite() && p.ear_l.is_finite());
        m = settled(Load::new(f64::NAN, f64::INFINITY, f64::NEG_INFINITY, true));
        assert!(m.head_offset.is_finite());
    }
    #[test]
    fn weightlessness_needs_airborne_and_near_zero_load_on_every_axis() {
        assert!(Preset::Float.load().floating());
        assert!(!Load::new(0., 0., 0., false).floating());
        assert!(!Load::new(0., 0., 1., true).floating());
        assert!(!Load::default().floating());
    }
    #[test]
    fn floppy_negative_g_ears_lift_past_horizontal_sway_and_settle_after_release() {
        let mut track = Track::default();
        let mut peak = 0_f32;
        let mut first = None;
        let mut changed = false;
        for frame in 0..180 {
            let time = frame as f32 / 60.;
            let m = track.update(Preset::NegativeExtreme.load(), 1. / 60.);
            let mut ordinary = Pose::default();
            ordinary.ear_l.z = -0.3;
            ordinary.ear_r.z = 0.3;
            m.apply(&mut ordinary, time);
            let mut floppy = ordinary;
            m.apply_floppy_ears(&mut floppy, time);
            assert!(floppy.ear_l.is_finite() && floppy.ear_r.is_finite());
            assert!(floppy.ear_l.abs().max_element() < 2.5);
            assert!(floppy.ear_r.abs().max_element() < 2.5);
            peak = peak.max(floppy.ear_l.z.abs());
            if frame == 60 {
                first = Some(floppy.ear_l);
            }
            if frame > 90 {
                assert!(floppy.ear_l.z.abs() > ordinary.ear_l.z.abs() + 0.3);
                changed |= floppy.ear_l.distance(first.unwrap()) > 0.05;
            }
        }
        assert!(peak > 1.8 && changed);
        let mut m = Motion::default();
        for _ in 0..120 {
            m = track.update(Load::default(), 1. / 60.);
        }
        assert!(m.ear_negative.abs() < 0.001);
        let mut neutral = Pose::default();
        m.apply_floppy_ears(&mut neutral, 5.);
        assert!(neutral.ear_l.length() < 0.001);
    }
}
