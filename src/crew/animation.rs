//! Small, allocation-free performance rig. Angles are radians; motion remains
//! readable at portrait size without dislocating the eyes or jaw.
use bevy::prelude::*;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Emotion {
    #[default]
    Content,
    Focused,
    Curious,
    Excited,
    Proud,
    Nervous,
    Afraid,
    Startled,
    Bracing,
    PositiveG,
    NegativeG,
    SideG,
    MissileAlarm,
    StallWorry,
    EngineConcern,
    Hurt,
    Dazed,
    Relieved,
    Celebrating,
    Sleepy,
}
impl Emotion {
    pub const ALL: [Self; 20] = [
        Self::Content,
        Self::Focused,
        Self::Curious,
        Self::Excited,
        Self::Proud,
        Self::Nervous,
        Self::Afraid,
        Self::Startled,
        Self::Bracing,
        Self::PositiveG,
        Self::NegativeG,
        Self::SideG,
        Self::MissileAlarm,
        Self::StallWorry,
        Self::EngineConcern,
        Self::Hurt,
        Self::Dazed,
        Self::Relieved,
        Self::Celebrating,
        Self::Sleepy,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Content => "CONTENT",
            Self::Focused => "FOCUSED",
            Self::Curious => "CURIOUS",
            Self::Excited => "EXCITED",
            Self::Proud => "PROUD",
            Self::Nervous => "NERVOUS",
            Self::Afraid => "AFRAID",
            Self::Startled => "STARTLED",
            Self::Bracing => "BRACING",
            Self::PositiveG => "POSITIVE G",
            Self::NegativeG => "NEGATIVE G",
            Self::SideG => "SIDE G",
            Self::MissileAlarm => "MISSILE ALERT",
            Self::StallWorry => "STALL WORRY",
            Self::EngineConcern => "ENGINE CONCERN",
            Self::Hurt => "HURT",
            Self::Dazed => "DAZED",
            Self::Relieved => "RELIEVED",
            Self::Celebrating => "CELEBRATING",
            Self::Sleepy => "SLEEPY",
        }
    }
    pub fn slug(self) -> String {
        self.label().to_ascii_lowercase().replace(' ', "-")
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.slug() == s)
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Content => "Soft breathing, relaxed ears and a gentle wandering gaze.",
            Self::Focused => "Leans into the task, narrows his eyes and checks the instruments.",
            Self::Curious => "Tilts his head, raises one brow and listens with an ear flick.",
            Self::Excited => "Bright eyes, bouncing ears and an eager little pant.",
            Self::Proud => "Lifts his chin, settles his shoulders and holds a confident smile.",
            Self::Nervous => "Glances between instruments, pins his ears and swallows.",
            Self::Afraid => "Recedes into his seat, eyes wide, jaw trembling and ears back.",
            Self::Startled => "Recoils sharply, opens his jaw, then cautiously recovers.",
            Self::Bracing => "Hunches against the harness, tenses his arms and squeezes his eyes.",
            Self::PositiveG => "Compresses into the seat, squints and grits his teeth under load.",
            Self::NegativeG => "Grimaces as his head lifts and his long ears float upward.",
            Self::SideG => "Leans opposite the signed lateral acceleration while his ears lag behind.",
            Self::MissileAlarm => "Scans toward the threat, raises his brows and breathes quickly.",
            Self::StallWorry => "Looks down at the instruments with a tense, uneven squint.",
            Self::EngineConcern => "Checks to one side with a questioning brow and worried mouth.",
            Self::Hurt => "A restrained asymmetric wince and short shoulder flinch.",
            Self::Dazed => "Slow blinks and a small, unsteady head wobble after a hard landing.",
            Self::Relieved => "Exhales, relaxes his ears and slowly opens his eyes again.",
            Self::Celebrating => "A delighted head bob, happy pant and buoyant ears after landing.",
            Self::Sleepy => {
                "Drooping eyelids, a slow nod and a gentle yawn while safely on the ground."
            }
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub head: Vec3,
    pub head_y: f32,
    pub chest: f32,
    pub breath: f32,
    pub ear_l: Vec3,
    pub ear_r: Vec3,
    pub gaze: Vec2,
    pub lids: Vec2,
    pub brow: Vec2,
    pub brow_angle: Vec2,
    pub jaw: f32,
    pub tongue: f32,
    pub brace: f32,
}
impl Default for Pose {
    fn default() -> Self {
        Self {
            head: Vec3::ZERO,
            head_y: 0.,
            chest: 0.,
            breath: 0.,
            ear_l: Vec3::ZERO,
            ear_r: Vec3::ZERO,
            gaze: Vec2::ZERO,
            lids: Vec2::splat(0.08),
            brow: Vec2::ZERO,
            brow_angle: Vec2::ZERO,
            jaw: 0.025,
            tongue: 0.,
            brace: 0.,
        }
    }
}
impl Pose {
    pub fn mix(self, b: Self, t: f32) -> Self {
        let t = t.clamp(0., 1.);
        let f = |a: f32, b: f32| a + (b - a) * t;
        Self {
            head: self.head.lerp(b.head, t),
            head_y: f(self.head_y, b.head_y),
            chest: f(self.chest, b.chest),
            breath: f(self.breath, b.breath),
            ear_l: self.ear_l.lerp(b.ear_l, t),
            ear_r: self.ear_r.lerp(b.ear_r, t),
            gaze: self.gaze.lerp(b.gaze, t),
            lids: self.lids.lerp(b.lids, t),
            brow: self.brow.lerp(b.brow, t),
            brow_angle: self.brow_angle.lerp(b.brow_angle, t),
            jaw: f(self.jaw, b.jaw),
            tongue: f(self.tongue, b.tongue),
            brace: f(self.brace, b.brace),
        }
    }
    pub fn closed() -> Self {
        Self {
            head: Vec3::new(0.16, 0., 0.04),
            head_y: -0.009,
            lids: Vec2::ONE,
            jaw: 0.,
            ..default()
        }
    }
}
/// Each clip has its own silhouette, cadence and asymmetric secondary motion.
/// Additive acceleration and blinks are applied by the driver after crossfade.
pub fn sample(e: Emotion, t: f32, seed: f32) -> Pose {
    use Emotion::*;
    let s = |rate: f32| (t * rate + seed).sin();
    let mut p = Pose {
        head: Vec3::new(s(1.1) * 0.008, s(0.6) * 0.018, s(0.8) * 0.009),
        breath: s(2.1) * 0.0018,
        ear_l: Vec3::new(s(1.5) * 0.012, 0., s(1.3) * 0.014),
        ear_r: Vec3::new(s(1.5) * 0.012, 0., -s(1.3) * 0.014),
        gaze: Vec2::new(s(0.6) * 0.12, s(0.8) * 0.04),
        ..default()
    };
    let mut ears = 0.;
    let mut pin = 0.;
    match e {
        Content => {
            p.jaw = 0.012;
            p.brow = Vec2::splat(0.001);
        }
        Focused => {
            p.head.x = -0.035;
            p.chest = -0.025;
            p.lids = Vec2::new(0.36, 0.30);
            p.gaze.y = -0.14 + 0.09 * s(1.1);
            p.brow_angle = Vec2::new(-0.10, 0.10);
            p.jaw = 0.012;
            pin = 0.08;
        }
        Curious => {
            p.head.z = 0.15 + 0.035 * s(1.5);
            p.head.y = 0.06 * s(0.8);
            p.brow = Vec2::new(0.010, -0.001);
            p.lids = Vec2::new(0.02, 0.20);
            p.ear_l.x += 0.09 * s(4.);
            p.gaze = Vec2::new(0.20 * s(0.8), 0.08);
            p.jaw = 0.045;
            ears = 0.06;
        }
        Excited => {
            p.head.x = -0.035 + 0.018 * s(8.);
            p.head_y = 0.004 * s(8.);
            p.brow = Vec2::splat(0.007);
            p.jaw = 0.19 + 0.035 * s(10.);
            p.tongue = 0.7;
            p.breath = s(7.) * 0.003;
            ears = 0.12 + 0.08 * s(8.);
        }
        Proud => {
            p.head.x = -0.075;
            p.head.z = -0.035;
            p.head_y = 0.006;
            p.lids = Vec2::splat(0.24);
            p.brow = Vec2::new(0.004, 0.008);
            p.jaw = 0.075;
            p.chest = 0.035;
            p.gaze.y = 0.12;
            ears = 0.03;
        }
        Nervous => {
            p.head.y = 0.045 * s(3.5);
            p.head.x = 0.035;
            p.gaze.x = 0.45 * s(4.5);
            p.gaze.y = -0.12;
            p.brow = Vec2::new(0.009, 0.004);
            p.brow_angle = Vec2::new(0.16, -0.16);
            p.jaw = 0.03 + 0.018 * s(7.).abs();
            p.breath = s(4.2) * 0.0028;
            pin = 0.23;
        }
        Afraid => {
            p.head.x = -0.05;
            p.head_y = -0.008;
            p.chest = 0.045;
            p.gaze = Vec2::new(0.22 * s(3.), 0.14);
            p.lids = Vec2::ZERO;
            p.brow = Vec2::splat(0.012);
            p.brow_angle = Vec2::new(0.18, -0.18);
            p.jaw = 0.20 + 0.018 * s(22.);
            p.brace = 0.55;
            pin = 0.38;
        }
        Startled => {
            let hit = (-t * 2.5).exp();
            p.head.x = -0.07 - 0.065 * hit;
            p.head.z = 0.025 * s(6.) * hit;
            p.head_y = 0.004;
            p.chest = 0.055 * hit;
            p.lids = Vec2::ZERO;
            p.brow = Vec2::splat(0.014);
            p.jaw = 0.12 + 0.20 * hit;
            p.brace = 0.75 * hit;
            ears = 0.10 + 0.22 * hit;
            pin = 0.12;
        }
        Bracing => {
            p.head.x = 0.075;
            p.head_y = -0.012;
            p.chest = -0.055;
            p.lids = Vec2::new(0.76, 0.84);
            p.brow = Vec2::splat(-0.005);
            p.brow_angle = Vec2::new(-0.18, 0.18);
            p.jaw = 0.035;
            p.brace = 0.95;
            pin = 0.30;
        }
        PositiveG => {
            p.head.x = 0.045;
            p.lids = Vec2::new(0.38, 0.44);
            p.brow = Vec2::splat(-0.004);
            p.brow_angle = Vec2::new(-0.16, 0.16);
            p.jaw = 0.035;
            p.brace = 0.7;
            p.breath = s(5.) * 0.001;
            pin = 0.16;
        }
        NegativeG => {
            p.head.x = -0.055;
            p.head_y = 0.008;
            p.lids = Vec2::new(0.32, 0.40);
            p.brow_angle = Vec2::new(0.13, -0.13);
            p.jaw = 0.095;
            p.brace = 0.4;
            ears = 0.30 + 0.03 * s(2.);
        }
        SideG => {
            p.head.z = 0.;
            p.chest = -0.015;
            p.lids = Vec2::new(0.28, 0.42);
            p.gaze.x = 0.;
            p.brow_angle = Vec2::new(-0.10, 0.08);
            p.brace = 0.5;
            p.jaw = 0.06;
        }
        MissileAlarm => {
            p.head.y = 0.10 + 0.12 * s(3.);
            p.head.x = -0.02;
            p.gaze.x = 0.48 * s(3.);
            p.brow = Vec2::splat(0.010);
            p.lids = Vec2::new(0., 0.04);
            p.jaw = 0.13 + 0.02 * s(8.);
            p.breath = s(6.) * 0.003;
            p.brace = 0.35;
            pin = 0.18;
        }
        StallWorry => {
            p.head.x = 0.065 + 0.018 * s(2.8);
            p.head.z = -0.045;
            p.gaze.y = -0.48;
            p.lids = Vec2::new(0.32, 0.12);
            p.brow = Vec2::new(0.006, -0.002);
            p.brow_angle = Vec2::new(0.13, -0.13);
            p.jaw = 0.065;
            p.brace = 0.45;
            pin = 0.14;
        }
        EngineConcern => {
            p.head.y = -0.13;
            p.head.z = 0.085;
            p.gaze = Vec2::new(-0.40, -0.18);
            p.brow = Vec2::new(0.011, -0.002);
            p.lids = Vec2::new(0.04, 0.36);
            p.jaw = 0.05;
            p.ear_r.x += 0.10 * s(2.);
            pin = 0.1;
        }
        Hurt => {
            let hit = (-t * 2.).exp();
            p.head = Vec3::new(0.06, -0.045, 0.095 + 0.045 * hit);
            p.head_y = -0.005;
            p.lids = Vec2::new(0.93, 0.48);
            p.brow = Vec2::new(-0.007, 0.003);
            p.jaw = 0.075;
            p.chest = -0.03 * hit;
            p.brace = 0.8;
            pin = 0.28;
        }
        Dazed => {
            p.head = Vec3::new(0.055 + 0.025 * s(2.), 0.045 * s(1.2), 0.065 * s(1.6));
            p.gaze = Vec2::new(0.28 * s(1.1), -0.15);
            p.lids = Vec2::new(0.56, 0.40);
            p.brow = Vec2::new(0.006, -0.001);
            p.jaw = 0.11;
            p.breath = s(1.4) * 0.002;
            pin = 0.14;
        }
        Relieved => {
            let exhale = (t * 0.8).min(1.);
            p.head.x = 0.035 * (1. - exhale);
            p.head_y = -0.004 * exhale;
            p.chest = 0.025 * exhale;
            p.lids = Vec2::splat(0.62 - 0.48 * exhale);
            p.brow = Vec2::splat(0.003);
            p.jaw = 0.10 * (1. - exhale) + 0.045;
            p.breath = s(1.5) * 0.003;
            pin = 0.12 * (1. - exhale);
        }
        Celebrating => {
            p.head.x = -0.04 + 0.035 * s(7.);
            p.head.z = 0.035 * s(3.5);
            p.head_y = 0.006 * s(7.);
            p.lids = Vec2::splat(0.23);
            p.brow = Vec2::splat(0.008);
            p.jaw = 0.21 + 0.025 * s(9.);
            p.tongue = 0.9;
            p.chest = 0.018 * s(3.5);
            ears = 0.14 + 0.07 * s(7.);
        }
        Sleepy => {
            let yawn = ((s(0.9) - 0.25) / 0.75).max(0.);
            p.head.x = 0.055 + 0.035 * s(0.8);
            p.head.z = 0.045;
            p.lids = Vec2::splat(0.66 + 0.18 * s(0.9));
            p.jaw = 0.03 + 0.24 * yawn;
            p.tongue = 0.25 * yawn;
            p.breath = s(1.3) * 0.002;
            pin = 0.07;
        }
    }
    p.ear_l += Vec3::new(pin, 0., -ears);
    p.ear_r += Vec3::new(pin, 0., ears);
    p
}
#[derive(Clone, Copy)]
pub struct Track {
    pub emotion: Emotion,
    pub age: f32,
    pub pose: Pose,
    from: Pose,
    pub transition: f32,
    initialized: bool,
}
impl Default for Track {
    fn default() -> Self {
        Self {
            emotion: default(),
            age: 0.,
            pose: default(),
            from: default(),
            transition: 0.,
            initialized: false,
        }
    }
}
impl Track {
    pub fn update(&mut self, e: Emotion, dt: f32, seed: f32) -> Pose {
        if self.emotion != e || !self.initialized {
            self.from = self.pose;
            self.emotion = e;
            self.age = 0.;
            self.transition = 0.;
            self.initialized = true;
        }
        self.age += dt;
        self.transition = (self.transition + dt / 0.22).min(1.);
        let u = self.transition * self.transition * (3. - 2. * self.transition);
        self.pose = self.from.mix(sample(e, self.age, seed), u);
        self.pose
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn vector(p: Pose) -> Vec<f32> {
        let mut a = vec![];
        a.extend(p.head.to_array());
        a.extend(p.gaze.to_array());
        a.extend(p.lids.to_array());
        a.extend(p.brow.to_array());
        a.extend(p.ear_l.to_array());
        a.extend(p.ear_r.to_array());
        a.extend([p.head_y, p.chest, p.breath, p.jaw, p.tongue, p.brace]);
        a
    }
    #[test]
    fn twenty_clips_are_distinct_finite_and_actually_move() {
        for (i, e) in Emotion::ALL.into_iter().enumerate() {
            let a = vector(sample(e, 0.3, 0.));
            let b = vector(sample(e, 1.1, 0.));
            assert!(
                a.iter().chain(&b).all(|x| x.is_finite() && x.abs() < 1.1),
                "{e:?}"
            );
            assert!(
                a.iter().zip(&b).any(|(a, b)| (a - b).abs() > 0.001),
                "static {e:?}"
            );
            for other in Emotion::ALL.into_iter().take(i) {
                assert_ne!(a, vector(sample(other, 0.3, 0.)), "duplicate {e:?}");
            }
        }
    }
    #[test]
    fn clip_transitions_do_not_snap_and_settle_in_under_quarter_second() {
        let mut t = Track::default();
        for _ in 0..60 {
            t.update(Emotion::Content, 1. / 60., 0.);
        }
        let before = t.pose;
        let first = t.update(Emotion::Afraid, 1. / 60., 0.);
        assert!((first.jaw - before.jaw).abs() < 0.01);
        for _ in 0..14 {
            t.update(Emotion::Afraid, 1. / 60., 0.);
        }
        assert_eq!(t.transition, 1.);
        assert!(t.pose.jaw > 0.15);
    }
}
