//! Gameplay G exposure and a layered pass-out performance. No flight forces or
//! medical thresholds live here: these timings are deliberately game tuning.
use super::{animation::Pose, loads::Load};
use bevy::prelude::*;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Alert,
    Struggling,
    Fading,
    Collapsing,
    Unconscious,
    Waking,
    Recovering,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    pub phase: Phase,
    pub age: f32,
    pub burden: f32,
    pub red: f32,
    safe: f32,
}
fn smooth(a: f32, b: f32, t: f32) -> f32 {
    let u = ((t - a) / (b - a)).clamp(0., 1.);
    u * u * (3. - 2. * u)
}
impl State {
    pub fn update(&mut self, load: Load, dt: f32) {
        // Bounded integration makes the stage timing independent of frame rate.
        let dt = dt.clamp(0., 60.);
        let steps = (dt * 60.).ceil().max(1.) as usize;
        let h = dt / steps as f32;
        for _ in 0..steps {
            self.tick(load, h);
        }
    }
    fn tick(&mut self, load: Load, dt: f32) {
        let positive = ((load.vertical - 4.5) / 4.5).max(0.).powf(1.6) * 0.42;
        let negative = ((-load.vertical - 1.3) / 1.7).max(0.).powf(1.4) * 0.30;
        let dose = positive.max(negative).min(0.95);
        let safe = (-0.8..=3.).contains(&load.vertical)
            && load.lateral.abs() < 2.5
            && load.forward.abs() < 2.5;
        self.age += dt;
        self.safe = if safe { self.safe + dt } else { 0. };
        if dose > 0. {
            self.red +=
                (if negative > positive { 1. } else { 0. } - self.red) * (1. - (-dt / 0.4).exp());
        }
        self.burden = (self.burden
            + dt * if dose > 0. {
                dose
            } else if safe {
                -0.18
            } else {
                -0.045
            })
        .clamp(0., 1.);
        match self.phase {
            Phase::Collapsing => {
                if self.age >= 1.15 {
                    self.enter(Phase::Unconscious);
                }
            }
            Phase::Unconscious => {
                if !safe {
                    self.burden = self.burden.max(0.8);
                }
                if self.age >= 4. && self.safe >= 0.8 {
                    self.enter(Phase::Waking);
                    self.burden = self.burden.min(0.65);
                }
            }
            Phase::Waking => {
                if self.burden >= 0.98 {
                    self.enter(Phase::Collapsing);
                } else if self.age >= 3.2 {
                    self.enter(Phase::Recovering);
                }
            }
            Phase::Recovering => {
                if self.burden >= 0.98 {
                    self.enter(Phase::Collapsing);
                } else if self.age >= 4. && self.burden < 0.22 {
                    self.enter(Phase::Alert);
                }
            }
            _ => {
                let next = if self.burden >= 0.999 {
                    Phase::Collapsing
                } else if self.burden >= 0.55 {
                    Phase::Fading
                } else if self.burden >= 0.22 {
                    Phase::Struggling
                } else {
                    Phase::Alert
                };
                if next != self.phase {
                    self.enter(next);
                }
            }
        }
    }
    fn enter(&mut self, p: Phase) {
        self.phase = p;
        self.age = 0.;
    }
    pub fn label(self) -> Option<&'static str> {
        Some(match self.phase {
            Phase::Alert => return None,
            Phase::Struggling => "G / STRUGGLING",
            Phase::Fading => "LOSING FOCUS",
            Phase::Collapsing => "PASSING OUT",
            Phase::Unconscious => "UNCONSCIOUS",
            Phase::Waking => "WAKING UP",
            Phase::Recovering => "RECOVERING",
        })
    }
    pub fn authority(self) -> f32 {
        match self.phase {
            Phase::Alert => 1.,
            Phase::Struggling | Phase::Fading => 1. - self.burden * 0.30,
            Phase::Collapsing => 0.7 * (1. - smooth(0., 0.45, self.age)),
            Phase::Unconscious => 0.,
            Phase::Waking => smooth(0.8, 3.2, self.age) * 0.65,
            Phase::Recovering => 0.65 + smooth(0., 4., self.age) * 0.35,
        }
    }
    pub fn responsive(self) -> bool {
        !matches!(
            self.phase,
            Phase::Collapsing | Phase::Unconscious | Phase::Waking
        )
    }
    pub fn weight(self) -> f32 {
        match self.phase {
            Phase::Alert => 0.,
            Phase::Struggling => smooth(0.15, 0.65, self.burden) * 0.65,
            Phase::Fading => 0.58 + smooth(0.55, 1., self.burden) * 0.32,
            Phase::Collapsing | Phase::Unconscious | Phase::Waking => 1.,
            Phase::Recovering => 1. - smooth(0., 4., self.age),
        }
    }
    pub fn vision(self) -> Vision {
        let dark = match self.phase {
            Phase::Alert | Phase::Struggling | Phase::Fading => smooth(0.4, 1., self.burden) * 0.42,
            Phase::Collapsing => 0.42 + 0.58 * smooth(0., 0.9, self.age),
            Phase::Unconscious => 1.,
            Phase::Waking => 1. - smooth(0.45, 3.2, self.age) * 0.87,
            Phase::Recovering => 0.13 * (1. - smooth(0., 3., self.age)),
        };
        let fatigue = match self.phase {
            Phase::Waking => 1. - smooth(0.8, 3.2, self.age) * 0.65,
            Phase::Recovering => 0.35 * (1. - smooth(0., 4., self.age)),
            _ => smooth(0.12, 0.9, self.burden),
        };
        Vision {
            grey: fatigue * (1. - self.red),
            red: fatigue * self.red,
            edge: fatigue.clamp(0., 1.),
            black: dark.clamp(0., 1.),
        }
    }
    /// Blend only the performance, retaining bounded inertial motion. Closed
    /// eyes cannot be reopened by chatter, alarms or the ordinary blink timer.
    pub fn apply(self, p: &mut Pose, time: f32, floppy: bool) -> Detail {
        let strength = self.weight();
        if strength == 0. {
            return Detail::default();
        }
        let s = |r: f32, o: f32| (time * r + o).sin();
        let breathing = s(1.55, 0.) * 0.0016;
        let limp_ears = if floppy {
            (Vec3::new(0.055, 0., 0.05), Vec3::new(0.065, 0., -0.04))
        } else {
            (Vec3::new(0.1, 0., 0.), Vec3::new(0.09, 0., 0.))
        };
        let mut out = Pose {
            head: Vec3::new(-0.115, -0.035, 0.15),
            head_y: -0.018,
            chest: -0.045,
            breath: breathing,
            ear_l: limp_ears.0,
            ear_r: limp_ears.1,
            gaze: Vec2::new(-0.03, -0.45),
            lids: Vec2::ONE,
            brow: Vec2::splat(-0.002),
            jaw: 0.055 + s(1.55, 0.3) * 0.006,
            tongue: 0.12,
            brace: 0.,
            ..default()
        };
        let mut d = Detail {
            arm_l: Vec3::new(-0.035, -0.015, -0.025),
            arm_r: Vec3::new(-0.045, 0.01, 0.03),
            wrist: Vec2::new(-0.16, -0.20),
            fingers: Vec2::new(-0.13, -0.16),
            head_offset: Vec3::new(0., 0., -0.008),
            ..default()
        };
        match self.phase {
            Phase::Struggling | Phase::Fading => {
                let fade = if self.phase == Phase::Fading {
                    smooth(0.55, 1., self.burden)
                } else {
                    0.
                };
                let fight = (s(3.4, 0.) * 0.5 + 0.5).powi(4);
                out.head = Vec3::new(
                    -0.015 - fade * 0.035 + fight * 0.025,
                    s(0.9, 0.) * 0.035,
                    s(1.7, 0.4) * 0.035,
                );
                out.head_y = -fade * 0.009;
                out.chest = -0.01 - fade * 0.015;
                out.gaze = Vec2::new(s(1.8, 0.) * 0.25, -0.12 - fade * 0.25);
                let flutter = s(12., 0.).max(0.) * 0.20 * fade;
                out.lids = Vec2::new(
                    0.27 + fade * 0.50 + flutter,
                    0.22 + fade * 0.53 + flutter * 0.8,
                )
                .min(Vec2::splat(0.94));
                out.brow = Vec2::new(0.003, 0.001) * (1. - fade);
                out.jaw = 0.03 + fade * 0.025 + s(17., 0.).abs() * fade * 0.008;
                out.ear_l = p.ear_l + Vec3::new(s(2., 0.) * 0.02, 0., 0.03 * fade);
                out.ear_r = p.ear_r + Vec3::new(s(2., 0.5) * 0.02, 0., -0.03 * fade);
                out.brace = (0.6 - fade * 0.5).max(0.);
                d.wrist *= fade;
                d.fingers *= fade;
                d.arm_l *= fade;
                d.arm_r *= fade;
                d.head_offset *= fade;
            }
            Phase::Collapsing => {
                let fall = smooth(0.10, 1., self.age);
                let recover_attempt = (self.age * 8.).sin() * (-self.age * 3.).exp() * 0.018;
                out.head = Vec3::new(
                    -0.045 - fall * 0.070 + recover_attempt,
                    s(0.9, 0.) * 0.035 * (1. - fall) - 0.035 * fall,
                    s(1.7, 0.4) * 0.035 * (1. - fall) + 0.15 * fall,
                );
                out.head_y = -0.009 - 0.009 * fall;
                out.chest = -0.025 - 0.020 * fall;
                out.gaze.y = -0.18 - fall * 0.27;
                out.lids = Vec2::new(
                    0.90 + 0.10 * smooth(0., 0.40, self.age),
                    0.86 + 0.14 * smooth(0.07, 0.50, self.age),
                );
                out.jaw = 0.040 + fall * 0.015;
                d.arm_l *= 0.9 + 0.1 * smooth(0.16, 0.75, self.age);
                d.arm_r *= 0.9 + 0.1 * smooth(0.28, 0.95, self.age);
                d.wrist *= 0.9 + 0.1 * fall;
                d.fingers *= 0.9 + 0.1 * fall;
                // Negative-G ear float survives loss of muscle tone.
                out.ear_l = p.ear_l.lerp(limp_ears.0, fall * 0.55);
                out.ear_r = p.ear_r.lerp(limp_ears.1, fall * 0.55);
                *p = p.mix(out, 0.9 + 0.1 * fall);
                return d;
            }
            Phase::Unconscious => {
                out.head.x += s(1.55, 0.7) * 0.003;
                out.head.z += s(0.8, 0.) * 0.004;
                out.ear_l += Vec3::new(s(1.55, 0.2) * 0.009, 0., s(1.1, 0.) * 0.012);
                out.ear_r += Vec3::new(s(1.55, 0.6) * 0.009, 0., -s(1.1, 0.5) * 0.012);
                // Retain some passive lift under continuing negative G.
                let settle = smooth(0., 0.55, self.age);
                out.ear_l =
                    (p.ear_l * 0.45 + limp_ears.0 * 0.55).lerp(out.ear_l + p.ear_l * 0.35, settle);
                out.ear_r =
                    (p.ear_r * 0.45 + limp_ears.1 * 0.55).lerp(out.ear_r + p.ear_r * 0.35, settle);
            }
            Phase::Waking => {
                let wake = smooth(0.5, 3.2, self.age);
                let searching = smooth(1.2, 2.7, self.age);
                let blink1 = 1. - smooth(0.60, 0.95, self.age) + smooth(1.13, 1.28, self.age)
                    - smooth(1.40, 1.78, self.age);
                out.lids = Vec2::new(
                    (0.75 - wake * 0.40).max(blink1),
                    (0.82 - wake * 0.48).max(blink1 * 0.97),
                )
                .clamp(Vec2::ZERO, Vec2::ONE);
                out.head = Vec3::new(
                    -0.115 * (1. - wake) + s(4., 0.) * (1. - wake) * 0.009,
                    -0.035 + s(1.4, 0.) * searching * 0.05,
                    0.15 * (1. - wake) + s(1.7, 0.) * 0.012,
                );
                out.head_y = -0.018 * (1. - wake);
                out.chest = -0.045 * (1. - wake);
                out.gaze = Vec2::new(s(2., 0.) * 0.25 * searching, -0.45 * (1. - wake));
                out.brow = Vec2::new(0.005, 0.002) * wake;
                out.tongue = 0.12 * (1. - wake);
                out.jaw = 0.055 - wake * 0.03
                    + smooth(1.4, 1.7, self.age) * (1. - smooth(1.8, 2.05, self.age)) * 0.025;
                out.ear_l = limp_ears.0.lerp(p.ear_l, wake * 0.6);
                out.ear_r = limp_ears.1.lerp(p.ear_r, wake * 0.6);
                d.arm_l *= 1. - wake * 0.6;
                d.arm_r *= 1. - wake * 0.6;
                d.wrist *= 1. - wake * 0.7;
                d.fingers *= 1. - wake * 0.65;
                d.head_offset *= 1. - wake;
            }
            Phase::Recovering => {
                let awake = smooth(0., 4., self.age);
                out = *p;
                out.head += Vec3::new(s(1.3, 0.) * 0.012, 0., 0.025 * (1. - awake));
                out.lids = out.lids.max(Vec2::splat(0.30 * (1. - awake)));
                let long_blink = (1. - ((self.age - 0.65) / 0.30).abs()).clamp(0., 1.);
                out.lids = out.lids.max(Vec2::splat(long_blink));
                out.jaw = 0.045 - smooth(0.1, 1.8, self.age) * 0.025;
                out.breath = s(1.9, 0.) * 0.003;
                out.brow += Vec2::new(0.003, 0.001) * (1. - awake);
                d.arm_l *= 0.4 * (1. - awake);
                d.arm_r *= 0.4 * (1. - awake);
                d.wrist *= 0.3 * (1. - awake);
                d.fingers *= 0.35 * (1. - awake);
                d.head_offset = Vec3::ZERO;
            }
            Phase::Alert => {}
        }
        *p = p.mix(out, strength);
        d.scaled(if matches!(self.phase, Phase::Struggling | Phase::Fading) {
            strength
        } else {
            1.
        })
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Vision {
    pub grey: f32,
    pub red: f32,
    pub edge: f32,
    pub black: f32,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Detail {
    pub arm_l: Vec3,
    pub arm_r: Vec3,
    pub wrist: Vec2,
    pub fingers: Vec2,
    pub head_offset: Vec3,
}
impl Detail {
    fn scaled(self, a: f32) -> Self {
        Self {
            arm_l: self.arm_l * a,
            arm_r: self.arm_r * a,
            wrist: self.wrist * a,
            fingers: self.fingers * a,
            head_offset: self.head_offset * a,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preview {
    Grey,
    Red,
    Collapse,
    Out,
    Wake,
    Recover,
    Sequence,
}
impl Preview {
    pub const ALL: [Self; 7] = [
        Self::Grey,
        Self::Red,
        Self::Collapse,
        Self::Out,
        Self::Wake,
        Self::Recover,
        Self::Sequence,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Grey => "GREY-OUT",
            Self::Red => "RED-OUT",
            Self::Collapse => "PASSING OUT",
            Self::Out => "UNCONSCIOUS",
            Self::Wake => "WAKING",
            Self::Recover => "RECOVERY",
            Self::Sequence => "FULL SEQUENCE",
        }
    }
    pub fn slug(self) -> &'static str {
        match self {
            Self::Grey => "faint-grey",
            Self::Red => "faint-red",
            Self::Collapse => "faint-collapse",
            Self::Out => "faint-out",
            Self::Wake => "faint-wake",
            Self::Recover => "faint-recover",
            Self::Sequence => "faint-sequence",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.slug() == s)
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Grey => {
                "Sustained positive G: grey-out, drifting gaze, eyelid flutter and a failing effort to brace."
            }
            Self::Red => {
                "Sustained negative G: red-out, fading focus and loosening hands beneath floating ears."
            }
            Self::Collapse => {
                "Eyes close unevenly, the last corrective head nod fails, then the harness catches a restrained slump."
            }
            Self::Out => {
                "Closed eyes, relaxed wrists/fingers, a slack jaw, slow breathing and passive ear movement."
            }
            Self::Wake => {
                "Uneven eyelid opening, a second blink, searching gaze, a small swallow and gradually returning hand tension."
            }
            Self::Recover => {
                "A tired long blink, steadying head, deeper exhale and a gradual return to the controls."
            }
            Self::Sequence => {
                "Full live-timed demonstration: G exposure, fading, collapse, unconsciousness, waking and recovery."
            }
        }
    }
    pub fn load(self) -> Load {
        match self {
            Self::Grey | Self::Collapse => Load::new(9., 0., 0., true),
            Self::Red => Load::new(-3., 0., 0., true),
            _ => Load::default(),
        }
    }
    pub fn snapshot(self, time: f32) -> State {
        let (phase, age, burden, red) = match self {
            Self::Grey => (Phase::Fading, 1.2 + time, 0.80, 0.),
            Self::Red => (Phase::Fading, 1.2 + time, 0.80, 1.),
            Self::Collapse => (Phase::Collapsing, time.min(1.15), 1., 0.),
            Self::Out => (Phase::Unconscious, time, 0.8, 0.),
            Self::Wake => (Phase::Waking, (time % 3.2).min(3.19), 0.45, 0.),
            Self::Recover => (Phase::Recovering, time.min(3.8), 0.1, 0.),
            Self::Sequence => (Phase::Alert, 0., 0., 0.),
        };
        State {
            phase,
            age,
            burden,
            red,
            safe: 0.,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Demo {
    pub age: f32,
    pub state: State,
}
impl Demo {
    pub fn update(&mut self, dt: f32) {
        self.age += dt;
        if self.age > 24. {
            *self = Self::default();
        }
        self.state.update(self.load(), dt);
    }
    pub fn load(self) -> Load {
        if (1.0..7.0).contains(&self.age) {
            Load::new(9., 0., 0., true)
        } else {
            Load::default()
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn impulse_does_not_cause_pass_out_but_sustained_signed_loads_do() {
        for g in [9., -3.] {
            let mut s = State::default();
            s.update(Load::new(g, 0., 0., true), 0.1);
            assert_eq!(s.phase, Phase::Alert);
            s.update(Load::new(g, 0., 0., true), 5.);
            assert_eq!(s.phase, Phase::Unconscious);
            assert_eq!(s.vision().black, 1.);
            assert_eq!(s.authority(), 0.);
        }
    }
    #[test]
    fn recovery_requires_safe_load_and_passes_through_waking_then_recovers() {
        let mut s = State::default();
        s.update(Load::new(9., 0., 0., true), 10.);
        assert_eq!(s.phase, Phase::Unconscious);
        s.update(Load::new(9., 0., 0., true), 10.);
        assert_eq!(s.phase, Phase::Unconscious);
        let mut saw_wake = false;
        for _ in 0..900 {
            s.update(Load::default(), 1. / 60.);
            saw_wake |= s.phase == Phase::Waking;
        }
        assert!(saw_wake);
        assert_eq!(s.phase, Phase::Alert);
        assert_eq!(s.authority(), 1.);
        assert_eq!(s.vision().black, 0.);
    }
    #[test]
    fn paused_time_freezes_exposure_and_all_snapshots_have_bounded_poses() {
        let mut s = State::default();
        s.update(Load::new(9., 0., 0., true), 0.);
        assert_eq!(s.burden, 0.);
        for preview in Preview::ALL {
            for frame in 0..300 {
                let time = frame as f32 / 60.;
                let state = preview.snapshot(time);
                let mut p = Pose::default();
                let detail = state.apply(&mut p, time, true);
                assert!(p.head.is_finite() && p.ear_l.is_finite() && detail.arm_l.is_finite());
                assert!(p.head.abs().max_element() < 0.4);
                assert!(p.lids.min_element() >= 0. && p.lids.max_element() <= 1.);
                let v = state.vision();
                assert!((0.0..=1.0).contains(&v.black));
            }
        }
    }
    #[test]
    fn sign_selects_grey_or_red_and_unconscious_eyes_stay_closed() {
        let mut positive = State::default();
        positive.update(Load::new(9., 0., 0., true), 1.8);
        let mut negative = State::default();
        negative.update(Load::new(-3., 0., 0., true), 2.5);
        assert!(positive.vision().grey > 0.3 && positive.vision().red < 0.01);
        assert!(negative.vision().red > 0.3);
        let mut p =
            super::super::animation::sample(super::super::animation::Emotion::Startled, 0.2, 0.);
        Preview::Out.snapshot(2.).apply(&mut p, 2., true);
        assert_eq!(p.lids, Vec2::ONE);
        assert_eq!(p.brace, 0.);
    }
}
