//! Cached organic dog-pilot meshes and an articulated facial/body rig.
use super::sculpt::{self, Rect3};
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
};
use std::collections::BTreeMap;
#[derive(Clone)]
pub struct Piece {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub joint: Joint,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Joint {
    Body,
    Chest,
    Head,
    Jaw,
    Tongue,
    EarL,
    EarR,
    EyeL,
    EyeR,
    LidL,
    LidR,
    LowerLidL,
    LowerLidR,
    BrowL,
    BrowR,
    ArmL,
    ArmR,
    WristL,
    WristR,
    FingersL,
    FingersR,
}
#[derive(Component)]
pub struct Bone {
    pub owner: usize,
    pub portrait: bool,
    pub joint: Joint,
    pub rest: Transform,
}
#[derive(Component)]
pub struct DogRoot;
const SKIN: usize = 0;
const TAN: usize = 1;
const BLACK: usize = 2;
const CLOTH: usize = 3;
const SEAM: usize = 4;
const WEB: usize = 5;
const METAL: usize = 6;
const SHELL: usize = 7;
const MOUTH: usize = 8;
const GUM: usize = 9;
const WHITE: usize = 10;
const IRIS: usize = 11;
const NOSE: usize = 12;
#[derive(Clone, Copy)]
struct Shape {
    width: f32,
    muzzle: f32,
    ear: f32,
    pointed: bool,
}
fn shape(b: usize) -> Shape {
    let (w, m, e, p) = match b {
        0 => (1., 0.235, 0.26, false),
        1 => (1., 0.19, 0.14, true),
        2 => (1.05, 0.22, 0.18, true),
        3 => (1.10, 0.18, 0.22, true),
        4 => (1.06, 0.235, 0.22, false),
        5 => (0.96, 0.26, 0.21, true),
        6 => (0.92, 0.20, 0.17, false),
        7 => (0.94, 0.30, 0.285, false),
        8 => (1.18, 0.145, 0.12, false),
        _ => (0.98, 0.22, 0.20, false),
    };
    Shape {
        width: w,
        muzzle: m,
        ear: e,
        pointed: p,
    }
}
pub fn floppy_ears(breed: usize) -> bool {
    !shape(breed).pointed
}
fn palette(c: &Appearance) -> [Color; 13] {
    let base = match c.coat {
        3 => ([0.055, 0.043, 0.034], [0.34, 0.17, 0.065]),
        4 => ([0.70, 0.60, 0.43], [0.90, 0.80, 0.59]),
        5 => ([0.50, 0.28, 0.09], [0.80, 0.51, 0.20]),
        6 => ([0.28, 0.32, 0.37], [0.60, 0.64, 0.67]),
        _ => match c.breed {
            1 | 3 => ([0.51, 0.23, 0.065], [0.80, 0.51, 0.22]),
            2 => ([0.20, 0.25, 0.29], [0.68, 0.70, 0.67]),
            4 => ([0.52, 0.32, 0.11], [0.79, 0.54, 0.24]),
            5 => ([0.15, 0.09, 0.055], [0.54, 0.30, 0.12]),
            6 => ([0.51, 0.45, 0.34], [0.72, 0.65, 0.51]),
            7 => ([0.19, 0.074, 0.028], [0.54, 0.27, 0.085]),
            8 => ([0.32, 0.30, 0.27], [0.68, 0.62, 0.50]),
            _ => ([0.36, 0.17, 0.065], [0.68, 0.40, 0.16]),
        },
    };
    let t = if c.coat == 1 {
        1.22
    } else if c.coat == 2 {
        0.65
    } else {
        1.
    };
    let rgb = |v: [f32; 3]| Color::srgb((v[0] * t).min(1.), (v[1] * t).min(1.), (v[2] * t).min(1.));
    [
        rgb(base.0),
        rgb(base.1),
        Color::srgb(0.012, 0.010, 0.009),
        if c.suit == 0 {
            Color::srgb(0.43, 0.19, 0.075)
        } else if c.suit == 1 {
            Color::srgb(0.16, 0.22, 0.16)
        } else {
            Color::srgb(0.045, 0.10, 0.17)
        },
        Color::srgb(0.26, 0.28, 0.23),
        Color::srgb(0.32, 0.35, 0.25),
        Color::srgb(0.36, 0.39, 0.38),
        Color::srgb(0.75, 0.73, 0.62),
        Color::srgb(0.065, 0.018, 0.018),
        Color::srgb(0.48, 0.16, 0.17),
        Color::srgb(0.88, 0.86, 0.73),
        if c.breed == 2 {
            Color::srgb(0.13, 0.37, 0.49)
        } else {
            Color::srgb(0.27, 0.13, 0.035)
        },
        Color::srgb(0.018, 0.013, 0.010),
    ]
}
struct Sculpt {
    groups: BTreeMap<(Joint, usize), Mesh>,
    sphere: Mesh,
    medium: Mesh,
    small: Mesh,
    cube: Mesh,
    tube: Mesh,
    colors: [Color; 13],
}
impl Sculpt {
    fn new(colors: [Color; 13]) -> Self {
        Self {
            groups: BTreeMap::new(),
            sphere: Sphere::new(1.).mesh().uv(40, 28),
            medium: Sphere::new(1.).mesh().uv(24, 16),
            small: Sphere::new(1.).mesh().uv(12, 8),
            cube: Cuboid::default().into(),
            tube: Cylinder::new(1., 1.).mesh().resolution(12).build(),
            colors,
        }
    }
    fn add(&mut self, j: Joint, m: usize, mut mesh: Mesh, t: Transform) {
        if mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_none() {
            let color = self.colors[m].to_linear().to_f32_array();
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![color; mesh.count_vertices()]);
        }
        let mesh = mesh.transformed_by(t);
        if let Some(out) = self.groups.get_mut(&(j, m)) {
            out.merge(&mesh).expect("pilot mesh attributes");
        } else {
            self.groups.insert((j, m), mesh);
        }
    }
    fn oval(&mut self, j: Joint, m: usize, p: Vec3, r: Vec3) {
        let mesh = if r.max_element() >= 0.10 {
            self.sphere.clone()
        } else if r.max_element() >= 0.025 {
            self.medium.clone()
        } else {
            self.small.clone()
        };
        self.add(j, m, mesh, Transform::from_translation(p).with_scale(r));
    }
    fn block(&mut self, j: Joint, m: usize, p: Vec3, r: Vec3, q: Quat) {
        self.add(
            j,
            m,
            self.cube.clone(),
            Transform::from_translation(p)
                .with_scale(r)
                .with_rotation(q),
        );
    }
    fn rod(&mut self, j: Joint, m: usize, a: Vec3, b: Vec3, r: f32) {
        let d = b - a;
        if d.length() < 1e-6 {
            return;
        }
        self.add(
            j,
            m,
            self.tube.clone(),
            Transform::from_translation((a + b) * 0.5)
                .with_scale(Vec3::new(r, d.length(), r))
                .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize())),
        );
    }
    fn curve(&mut self, j: Joint, m: usize, p: impl IntoIterator<Item = Vec3>, r: f32) {
        let p: Vec<_> = p.into_iter().collect();
        for pair in p.windows(2) {
            self.rod(j, m, pair[0], pair[1], r);
        }
    }
    fn strap(&mut self, j: Joint, a: Vec3, b: Vec3, width: f32) {
        let d = b - a;
        let q = Quat::from_rotation_arc(Vec3::Y, d.normalize());
        self.block(
            j,
            WEB,
            (a + b) * 0.5,
            Vec3::new(width, d.length(), 0.007),
            q,
        );
        for side in [-1., 1.] {
            let edge = q * Vec3::X * (width * 0.38) * side;
            for i in 0..14 {
                let t = i as f32 / 14.;
                self.rod(
                    j,
                    SEAM,
                    a.lerp(b, t) + edge - Vec3::Z * 0.004,
                    a.lerp(b, (t + 0.025).min(1.)) + edge - Vec3::Z * 0.004,
                    0.0006,
                );
            }
        }
    }
}
fn lid(upper: bool) -> Mesh {
    let (rows, cols) = (18, 32);
    let mut p = Vec::new();
    let mut n = Vec::new();
    let mut uv = Vec::new();
    let mut idx = Vec::new();
    for j in 0..=rows {
        let theta = j as f32 / rows as f32 * std::f32::consts::FRAC_PI_2;
        for i in 0..=cols {
            let phi = (i as f32 / cols as f32 - 0.5) * std::f32::consts::PI;
            let v = Vec3::new(
                0.049 * theta.sin() * phi.sin(),
                0.064 * theta.cos() * if upper { 1. } else { -1. },
                -0.058 * theta.sin() * phi.cos(),
            );
            p.push(v.to_array());
            n.push(
                (v / Vec3::new(0.049, 0.064, 0.058).powf(2.))
                    .normalize_or(Vec3::Y)
                    .to_array(),
            );
            uv.push([i as f32 / cols as f32, j as f32 / rows as f32]);
        }
    }
    for j in 0..rows {
        for i in 0..cols {
            let a = (j * (cols + 1) + i) as u32;
            let b = a + (cols + 1) as u32;
            let tri = if upper {
                [a, a + 1, b, a + 1, b + 1, b]
            } else {
                [a, b, a + 1, a + 1, b, b + 1]
            };
            idx.extend(tri);
        }
    }
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, n);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.insert_indices(Indices::U32(idx));
    m
}
fn ear(sh: Shape, side: f32) -> Mesh {
    let (rows, cols) = (30, 32);
    let mut p = Vec::new();
    let mut n = Vec::new();
    let mut uv = Vec::new();
    let mut idx = Vec::new();
    for j in 0..=rows {
        let t = j as f32 / rows as f32;
        let width = if sh.pointed {
            0.064 * (1. - t).powf(0.8) + 0.002
        } else {
            0.002 + 0.061 * (std::f32::consts::PI * t).sin().max(0.).powf(0.65)
        };
        let thick = 0.003 + 0.014 * (std::f32::consts::PI * t).sin();
        for i in 0..=cols {
            let a = i as f32 / cols as f32 * std::f32::consts::TAU;
            let x = side * 0.018 * (std::f32::consts::PI * t).sin() + width * a.cos();
            let y = sh.ear * t * if sh.pointed { 1. } else { -1. };
            let z = -0.012 - 0.025 * (std::f32::consts::PI * t).sin() + thick * a.sin();
            p.push([x, y, z]);
            n.push(
                Vec3::new(
                    a.cos() / width,
                    if j == 0 { 0.2 } else { 0. },
                    a.sin() / thick,
                )
                .normalize()
                .to_array(),
            );
            uv.push([i as f32 / cols as f32, t]);
        }
    }
    for j in 0..rows {
        for i in 0..cols {
            let a = (j * (cols + 1) + i) as u32;
            let b = a + (cols + 1) as u32;
            if sh.pointed {
                idx.extend([a, b, a + 1, a + 1, b, b + 1]);
            } else {
                idx.extend([a, a + 1, b, a + 1, b + 1, b]);
            }
        }
    }
    // Tiny end caps close the loft without coplanar overlapping ear shells.
    for row in [0, rows] {
        let start = (row * (cols + 1)) as u32;
        for i in 1..cols - 1 {
            idx.extend([start, start + i as u32, start + i as u32 + 1]);
        }
    }
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, n);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.insert_indices(Indices::U32(idx));
    m
}
pub fn build(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    _images: &mut Assets<Image>,
    config: &Appearance,
) -> Vec<Piece> {
    use Joint::*;
    let sh = shape(config.breed);
    let colors = palette(config);
    let mut s = Sculpt::new(colors);
    let w = sh.width;
    // Full seated body with fitted fabric, separate shoulders and a reclined back.
    s.oval(
        Body,
        CLOTH,
        Vec3::new(0., 0.055, 0.025),
        Vec3::new(0.139, 0.085, 0.12),
    );
    s.oval(
        Chest,
        CLOTH,
        Vec3::new(0., 0.28, 0.025),
        Vec3::new(0.133, 0.196, 0.091),
    );
    s.oval(
        Chest,
        SEAM,
        Vec3::new(0., 0.455, 0.012),
        Vec3::new(0.088, 0.025, 0.067),
    );
    s.oval(
        Chest,
        SKIN,
        Vec3::new(0., 0.495, 0.014),
        Vec3::new(0.064, 0.055, 0.063),
    );
    for side in [-1., 1.] {
        s.rod(
            Body,
            CLOTH,
            Vec3::new(side * 0.08, 0.05, -0.01),
            Vec3::new(side * 0.105, -0.025, -0.30),
            0.067,
        );
        s.oval(
            Body,
            SEAM,
            Vec3::new(side * 0.105, -0.035, -0.30),
            Vec3::new(0.062, 0.050, 0.067),
        );
        s.rod(
            Body,
            CLOTH,
            Vec3::new(side * 0.105, -0.06, -0.30),
            Vec3::new(side * 0.105, -0.26, -0.40),
            0.045,
        );
        s.oval(
            Body,
            BLACK,
            Vec3::new(side * 0.105, -0.27, -0.45),
            Vec3::new(0.056, 0.056, 0.10),
        );
        s.block(
            Body,
            BLACK,
            Vec3::new(side * 0.105, -0.314, -0.45),
            Vec3::new(0.115, 0.018, 0.20),
            Quat::IDENTITY,
        );
        for i in 0..6 {
            s.rod(
                Body,
                SEAM,
                Vec3::new(side * 0.105 - 0.020, -0.229, -0.49 + i as f32 * 0.018),
                Vec3::new(side * 0.105 + 0.020, -0.229, -0.49 + i as f32 * 0.018),
                0.0015,
            );
        }
        let arm = if side < 0. { ArmL } else { ArmR };
        s.oval(arm, CLOTH, Vec3::ZERO, Vec3::new(0.063, 0.070, 0.066));
        s.rod(
            arm,
            CLOTH,
            Vec3::new(0., -0.01, 0.),
            Vec3::new(side * 0.055, -0.175, -0.05),
            0.048,
        );
        s.oval(
            arm,
            SEAM,
            Vec3::new(side * 0.055, -0.17, -0.05),
            Vec3::new(0.052, 0.042, 0.047),
        );
        s.rod(
            arm,
            CLOTH,
            Vec3::new(side * 0.055, -0.17, -0.05),
            Vec3::new(side * 0.045, -0.13, -0.27),
            0.039,
        );
        let wrist = if side < 0. { WristL } else { WristR };
        let fingers = if side < 0. { FingersL } else { FingersR };
        s.oval(wrist, BLACK, Vec3::ZERO, Vec3::new(0.040, 0.032, 0.051));
        for i in 0..4 {
            s.oval(
                fingers,
                SEAM,
                Vec3::new((i as f32 - 1.5) * 0.015, 0., -0.016),
                Vec3::new(0.008, 0.017, 0.021),
            );
        }
        // Embroidered paw patch: one pad and four toes on an actual sleeve.
        s.oval(
            arm,
            WEB,
            Vec3::new(side * 0.056, 0.003, -0.015),
            Vec3::new(0.009, 0.030, 0.026),
        );
        s.oval(
            arm,
            SHELL,
            Vec3::new(side * 0.065, -0.004, -0.015),
            Vec3::new(0.0025, 0.011, 0.010),
        );
        for i in 0..4 {
            let a = (i as f32 - 1.5) * 0.55;
            s.oval(
                arm,
                SHELL,
                Vec3::new(
                    side * 0.066,
                    0.008 + a.cos() * 0.007,
                    -0.015 + a.sin() * 0.017,
                ),
                Vec3::new(0.002, 0.004, 0.003),
            );
        }
        s.strap(
            Chest,
            Vec3::new(side * 0.094, 0.43, -0.049),
            Vec3::new(side * 0.047, 0.102, -0.096),
            0.031,
        );
        s.strap(
            Body,
            Vec3::new(side * 0.13, 0.09, -0.081),
            Vec3::new(side * 0.02, 0.09, -0.113),
            0.030,
        );
        // Buckles are four-sided frames rather than large floating gray blocks.
        let b = Vec3::new(side * 0.079, 0.32, -0.091);
        for (a, c) in [
            (Vec3::new(-0.017, -0.022, 0.), Vec3::new(0.017, -0.022, 0.)),
            (Vec3::new(-0.017, 0.022, 0.), Vec3::new(0.017, 0.022, 0.)),
            (Vec3::new(-0.017, -0.022, 0.), Vec3::new(-0.017, 0.022, 0.)),
            (Vec3::new(0.017, -0.022, 0.), Vec3::new(0.017, 0.022, 0.)),
        ] {
            s.rod(Chest, METAL, b + a, b + c, 0.0025);
        }
        s.block(
            Chest,
            SEAM,
            Vec3::new(side * 0.079, 0.235, -0.075),
            Vec3::new(0.045, 0.071, 0.014),
            Quat::from_rotation_z(side * 0.1),
        );
        for i in 0..9 {
            s.rod(
                Chest,
                WEB,
                Vec3::new(side * 0.080 - 0.014, 0.207 + i as f32 * 0.006, -0.084),
                Vec3::new(side * 0.080 - 0.010, 0.207 + i as f32 * 0.006, -0.084),
                0.00065,
            );
        }
    }
    s.oval(
        Body,
        METAL,
        Vec3::new(0., 0.095, -0.116),
        Vec3::new(0.023, 0.023, 0.005),
    );
    s.oval(
        Body,
        BLACK,
        Vec3::new(0., 0.095, -0.122),
        Vec3::new(0.014, 0.014, 0.002),
    );
    s.rod(
        Chest,
        BLACK,
        Vec3::new(0., 0.14, -0.074),
        Vec3::new(0., 0.43, -0.058),
        0.004,
    );
    for i in 0..22 {
        s.block(
            Chest,
            METAL,
            Vec3::new(0., 0.14 + i as f32 * 0.013, -0.080 + i as f32 * 0.0007),
            Vec3::new(0.007, 0.002, 0.002),
            Quat::IDENTITY,
        );
    }
    s.oval(
        Chest,
        METAL,
        Vec3::new(0., 0.398, -0.066),
        Vec3::new(0.007, 0.013, 0.002),
    );
    // A single continuous sculpt joins the skull, cheeks, bridge and muzzle.
    let muzzle = sh.muzzle;
    let tip = -0.09 - muzzle;
    let sdf = |p: Vec3| {
        let e = |c: Vec3, r: Vec3| sculpt::ellipsoid(p, c, r);
        let mut d = e(
            Vec3::new(0., 0.018, 0.012),
            Vec3::new(0.146 * w, 0.174, 0.137),
        );
        for side in [-1., 1.] {
            d = sculpt::union(
                d,
                e(
                    Vec3::new(side * 0.073 * w, -0.048, -0.050),
                    Vec3::new(0.078 * w, 0.084, 0.092),
                ),
                0.045,
            );
        }
        d = sculpt::union(
            d,
            e(
                Vec3::new(0., -0.029, -0.10),
                Vec3::new(0.080 * w, 0.079, muzzle * 0.56),
            ),
            0.042,
        );
        d = sculpt::union(
            d,
            e(
                Vec3::new(0., -0.069, -0.09 - muzzle * 0.55),
                Vec3::new(0.079 * w, 0.054, muzzle * 0.55),
            ),
            0.033,
        );
        for side in [-1., 1.] {
            d = sculpt::union(
                d,
                e(
                    Vec3::new(side * 0.034 * w, -0.070, tip + 0.035),
                    Vec3::new(0.046 * w, 0.039, 0.049),
                ),
                0.018,
            );
        }
        if p.z < -0.11 {
            d = d.max(-p.y - 0.118);
        }
        for side in [-1., 1.] {
            d = sculpt::subtract(
                d,
                e(
                    Vec3::new(side * 0.066 * w, 0.037, -0.126),
                    Vec3::new(0.051, 0.065, 0.050),
                ),
                0.008,
            );
        }
        d
    };
    let fur = colors[SKIN].to_linear().to_vec3();
    let tan = colors[TAN].to_linear().to_vec3();
    let cream = Vec3::new(0.59, 0.48, 0.31);
    let color = |p: Vec3| {
        let snout = sculpt::smooth(-0.10, -0.24, p.z) * sculpt::smooth(-0.015, -0.07, p.y);
        let brows = [-1., 1.].map(|side| {
            let d =
                (p - Vec3::new(side * 0.068 * w, 0.108, -0.099)) / Vec3::new(0.040, 0.024, 0.037);
            (1. - d.length()).clamp(0., 1.)
        });
        let cheek = sculpt::smooth(0.035, -0.055, p.y) * sculpt::smooth(-0.01, -0.12, p.z) * 0.6;
        let mut c = fur.lerp(tan, snout.max(cheek).max(brows[0].max(brows[1])));
        let blaze = if config.marking == 1 || matches!(config.breed, 0 | 1 | 2 | 3 | 6 | 9) {
            (1. - (p.x / 0.023).abs()).clamp(0., 1.) * sculpt::smooth(0.16, -0.07, p.y)
        } else {
            0.
        };
        let patch = if config.marking == 2 {
            (1. - ((p - Vec3::new(-0.082, 0.06, -0.10)) / Vec3::new(0.065, 0.075, 0.08)).length())
                .clamp(0., 1.)
        } else {
            0.
        };
        c = c.lerp(cream, blaze.max(patch) * 0.9);
        let grain =
            (p.x * 834. + p.y * 713. + p.z * 527.).sin() * (p.y * 433. - p.z * 311.).sin() * 0.018;
        (c * (1. + grain)).extend(1.).to_array()
    };
    s.add(
        Head,
        SKIN,
        sculpt::surface(
            Rect3 {
                min: Vec3::new(-0.22 * w, -0.19, tip - 0.05),
                max: Vec3::new(0.22 * w, 0.215, 0.17),
            },
            0.008,
            sdf,
            color,
        ),
        Transform::default(),
    );
    // Wet triangular canine nose; the nostrils are dark recessed slits.
    let mut nose = s.sphere.clone();
    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(v)) =
        nose.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for p in v {
            p[0] *= 0.73 + 0.27 * (p[1] * 0.5 + 0.5);
        }
    }
    nose.compute_smooth_normals();
    s.add(
        Head,
        NOSE,
        nose,
        Transform::from_xyz(0., -0.054, tip - 0.005).with_scale(Vec3::new(0.047 * w, 0.029, 0.027)),
    );
    for side in [-1., 1.] {
        s.add(
            Head,
            BLACK,
            s.small.clone(),
            Transform::from_xyz(side * 0.022 * w, -0.058, tip - 0.031)
                .with_scale(Vec3::new(0.011, 0.004, 0.0015))
                .with_rotation(Quat::from_rotation_z(side * 0.24)),
        );
    }
    s.rod(
        Head,
        BLACK,
        Vec3::new(0., -0.079, tip - 0.016),
        Vec3::new(0., -0.104, tip + 0.010),
        0.0015,
    );
    // Upper mouth cavity and a compact lower mandible, not a flat opening bib.
    s.oval(
        Head,
        MOUTH,
        Vec3::new(0., -0.115, -0.10 - muzzle * 0.49),
        Vec3::new(0.071 * w, 0.009, muzzle * 0.49),
    );
    s.oval(
        Jaw,
        TAN,
        Vec3::new(0., -0.014, -muzzle * 0.47),
        Vec3::new(0.063 * w, 0.019, muzzle * 0.50),
    );
    s.oval(
        Jaw,
        MOUTH,
        Vec3::new(0., 0.005, -muzzle * 0.47),
        Vec3::new(0.062 * w, 0.006, muzzle * 0.46),
    );
    s.oval(
        Tongue,
        GUM,
        Vec3::new(0., 0.012, -muzzle * 0.78),
        Vec3::new(0.024, 0.0055, 0.060),
    );
    s.rod(
        Tongue,
        MOUTH,
        Vec3::new(0., 0.016, -muzzle * 0.65),
        Vec3::new(0., 0.016, -muzzle * 0.93),
        0.00065,
    );
    for side in [-1., 1.] {
        for i in 0..4 {
            s.oval(
                Jaw,
                WHITE,
                Vec3::new(
                    side * 0.049
                        * w
                        * (1. - ((0.07 - i as f32 * 0.17) / 0.50).powi(2))
                            .max(0.)
                            .sqrt(),
                    0.002,
                    -muzzle * (0.40 + i as f32 * 0.17),
                ),
                Vec3::new(0.004, 0.005, 0.006),
            );
        }
        s.curve(
            Jaw,
            BLACK,
            (0..18).map(|i| {
                let t = i as f32 / 17.;
                Vec3::new(
                    side * 0.063 * w * (std::f32::consts::PI * t).sin().max(0.).powf(0.35),
                    -0.003,
                    -muzzle * t,
                )
            }),
            0.0014,
        );
        for i in 0..5 {
            s.oval(
                Head,
                BLACK,
                Vec3::new(
                    side * (0.027 + (i % 3) as f32 * 0.012),
                    -0.071 - (i / 3) as f32 * 0.012,
                    tip - 0.001 + (i % 3) as f32 * 0.009,
                ),
                Vec3::splat(0.0014),
            );
        }
        for i in 0..2 {
            s.rod(
                Head,
                SEAM,
                Vec3::new(side * 0.060, -0.080 - i as f32 * 0.013, tip + 0.028),
                Vec3::new(
                    side * (0.115 + i as f32 * 0.013),
                    -0.082 - i as f32 * 0.015,
                    tip + 0.048,
                ),
                0.00045,
            );
        }
    }
    for (side, eye, upper, lower, brow, ear_joint) in [
        (-1., EyeL, LidL, LowerLidL, BrowL, EarL),
        (1., EyeR, LidR, LowerLidR, BrowR, EarR),
    ] {
        let at = Vec3::new(side * 0.066 * w, 0.037, -0.126);
        s.oval(Head, SKIN, at, Vec3::new(0.049, 0.060, 0.035));
        s.oval(
            Head,
            WHITE,
            at - Vec3::Z * 0.002,
            Vec3::new(0.047, 0.057, 0.035),
        );
        s.oval(eye, IRIS, Vec3::ZERO, Vec3::new(0.025, 0.032, 0.005));
        s.oval(
            eye,
            BLACK,
            Vec3::new(0., 0., -0.0045),
            Vec3::new(0.012, 0.020, 0.002),
        );
        s.oval(
            eye,
            WHITE,
            Vec3::new(-0.007, 0.011, -0.006),
            Vec3::new(0.0055, 0.0065, 0.001),
        );
        s.oval(
            eye,
            WHITE,
            Vec3::new(0.006, -0.007, -0.006),
            Vec3::splat(0.002),
        );
        s.add(upper, SKIN, lid(true), Transform::default());
        let mut lower_lid = lid(false);
        lower_lid.insert_attribute(
            Mesh::ATTRIBUTE_COLOR,
            vec![
                colors[SKIN]
                    .to_linear()
                    .to_vec3()
                    .lerp(colors[TAN].to_linear().to_vec3(), 0.28)
                    .extend(1.)
                    .to_array();
                lower_lid.count_vertices()
            ],
        );
        s.add(lower, SKIN, lower_lid, Transform::default());
        s.curve(
            upper,
            BLACK,
            (0..25).map(|i| {
                let a = (i as f32 / 24. - 0.5) * std::f32::consts::PI;
                Vec3::new(0.049 * a.sin(), 0., -0.058 * a.cos())
            }),
            0.0012,
        );
        s.curve(
            brow,
            TAN,
            (0..13).map(|i| {
                let t = i as f32 / 12.;
                Vec3::new(
                    (t - 0.5) * 0.062,
                    0.004 * (std::f32::consts::PI * t).sin(),
                    -0.002,
                )
            }),
            0.005,
        );
        s.add(ear_joint, SKIN, ear(sh, side), Transform::default());
    }
    // Tailored open helmet shell with a flat crown stripe and rubber seal.
    if config.equipment != 1 {
        let (rows, cols) = (22, 64);
        let mut p = Vec::new();
        let mut n = Vec::new();
        let mut uv = Vec::new();
        let mut idx = Vec::new();
        for j in 0..=rows {
            let t = j as f32 / rows as f32;
            for i in 0..=cols {
                let a = i as f32 / cols as f32 * std::f32::consts::TAU;
                let theta = t * (1.55 - (-a.sin()).max(0.) * 0.55);
                let v = Vec3::new(
                    0.163 * w * theta.sin() * a.cos(),
                    0.189 * theta.cos() + 0.038,
                    0.158 * theta.sin() * a.sin() + 0.025,
                );
                p.push(v.to_array());
                n.push(
                    Vec3::new(
                        v.x / (0.163 * w).powi(2),
                        (v.y - 0.038) / 0.189_f32.powi(2),
                        (v.z - 0.025) / 0.158_f32.powi(2),
                    )
                    .normalize()
                    .to_array(),
                );
                uv.push([i as f32 / cols as f32, t]);
            }
        }
        for j in 0..rows {
            for i in 0..cols {
                let a = (j * (cols + 1) + i) as u32;
                let b = a + (cols + 1) as u32;
                idx.extend([a, a + 1, b, a + 1, b + 1, b]);
            }
        }
        let mut helmet = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        helmet.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
        helmet.insert_attribute(Mesh::ATTRIBUTE_NORMAL, n);
        helmet.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        helmet.insert_indices(Indices::U32(idx));
        s.add(Head, SHELL, helmet, Transform::default());
        s.curve(
            Head,
            BLACK,
            (0..65).map(|i| {
                let a = i as f32 / 64. * std::f32::consts::TAU;
                let t = 1.55 - (-a.sin()).max(0.) * 0.55;
                Vec3::new(
                    0.164 * w * t.sin() * a.cos(),
                    0.189 * t.cos() + 0.038,
                    0.159 * t.sin() * a.sin() + 0.025,
                )
            }),
            0.0055,
        );
        // Continuous flush crown stripe, with no gaps between block segments.
        let mut bp = Vec::new();
        let mut bn = Vec::new();
        let mut buv = Vec::new();
        let mut bi = Vec::new();
        for i in 0..=48 {
            let a = -0.99 + i as f32 / 48. * 2.38;
            let n = Vec3::new(0., a.cos(), a.sin());
            for side in [-1., 1.] {
                bp.push([
                    side * 0.009,
                    0.191 * a.cos() + 0.038,
                    0.160 * a.sin() + 0.025,
                ]);
                bn.push(n.to_array());
                buv.push([(side + 1.) / 2., i as f32 / 48.]);
            }
            if i < 48 {
                let j = (i * 2) as u32;
                bi.extend([j, j + 2, j + 1, j + 1, j + 2, j + 3]);
            }
        }
        let mut band = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        band.insert_attribute(Mesh::ATTRIBUTE_POSITION, bp);
        band.insert_attribute(Mesh::ATTRIBUTE_NORMAL, bn);
        band.insert_attribute(Mesh::ATTRIBUTE_UV_0, buv);
        band.insert_indices(Indices::U32(bi));
        s.add(Head, WEB, band, Transform::default());
    } else {
        s.curve(
            Head,
            BLACK,
            (0..25).map(|i| {
                let a = i as f32 / 24. * std::f32::consts::PI;
                Vec3::new(a.cos() * 0.166 * w, a.sin() * 0.198 + 0.015, 0.03)
            }),
            0.009,
        );
    }
    for side in [-1., 1.] {
        s.oval(
            Head,
            BLACK,
            Vec3::new(side * 0.159 * w, 0.025, 0.015),
            Vec3::new(0.020, 0.055, 0.047),
        );
        s.oval(
            Head,
            METAL,
            Vec3::new(side * 0.174 * w, 0.025, 0.015),
            Vec3::new(0.009, 0.045, 0.037),
        );
        s.oval(
            Head,
            WEB,
            Vec3::new(side * 0.181 * w, 0.025, 0.015),
            Vec3::new(0.006, 0.035, 0.027),
        );
        for i in 0..6 {
            let a = i as f32 / 6. * std::f32::consts::TAU;
            s.oval(
                Head,
                BLACK,
                Vec3::new(
                    side * 0.187 * w,
                    0.025 + a.cos() * 0.030,
                    0.015 + a.sin() * 0.022,
                ),
                Vec3::splat(0.002),
            );
        }
    }
    s.curve(
        Head,
        METAL,
        [
            Vec3::new(-0.177 * w, -0.005, 0.012),
            Vec3::new(-0.18 * w, -0.072, -0.10),
            Vec3::new(-0.105, -0.10, tip + 0.026),
        ],
        0.0035,
    );
    s.oval(
        Head,
        BLACK,
        Vec3::new(-0.105, -0.10, tip + 0.026),
        Vec3::new(0.018, 0.008, 0.009),
    );
    if config.equipment == 2 {
        s.oval(
            Head,
            WEB,
            Vec3::new(0., -0.083, tip - 0.013),
            Vec3::new(0.072, 0.059, 0.044),
        );
        s.oval(
            Head,
            BLACK,
            Vec3::new(0., -0.087, tip - 0.054),
            Vec3::new(0.036, 0.027, 0.010),
        );
        for i in 0..4 {
            s.block(
                Head,
                METAL,
                Vec3::new(0., -0.103 + i as f32 * 0.009, tip - 0.064),
                Vec3::new(0.041, 0.002, 0.002),
                Quat::IDENTITY,
            );
        }
        for i in 0..26 {
            let t = i as f32 / 25.;
            s.oval(
                Chest,
                BLACK,
                Vec3::new(0.05 + 0.08 * t, 0.50 - 0.31 * t, -0.40 + 0.33 * t),
                Vec3::splat(0.014),
            );
            s.oval(
                Chest,
                SEAM,
                Vec3::new(0.05 + 0.08 * t, 0.50 - 0.31 * t, -0.40 + 0.33 * t),
                Vec3::new(0.015, 0.003, 0.015),
            );
        }
    }
    if matches!(config.breed, 6 | 9) {
        for i in 0..26 {
            let a = i as f32 * 2.39996;
            let p = Vec3::new(
                a.cos() * 0.12,
                -0.045 + (i % 7) as f32 * 0.022,
                -0.06 + a.sin() * 0.055,
            );
            s.rod(
                Head,
                TAN,
                p,
                p + Vec3::new(a.cos() * 0.019, 0.011, -0.006),
                0.003,
            );
        }
    }
    let mats: Vec<_> = (0..colors.len())
        .map(|i| {
            materials.add(StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: if i == NOSE {
                    0.27
                } else if matches!(i, IRIS | WHITE) {
                    0.23
                } else if i == METAL {
                    0.38
                } else {
                    0.87
                },
                metallic: if i == METAL { 0.55 } else { 0. },
                reflectance: if matches!(i, NOSE | IRIS | WHITE) {
                    0.42
                } else {
                    0.19
                },
                cull_mode: None,
                ..default()
            })
        })
        .collect();
    s.groups
        .into_iter()
        .map(|((joint, m), mesh)| Piece {
            joint,
            mesh: meshes.add(mesh),
            material: mats[m].clone(),
        })
        .collect()
}
fn rest(j: Joint, breed: usize) -> Transform {
    use Joint::*;
    let sh = shape(breed);
    let w = sh.width;
    match j {
        Body | Chest | Tongue => Transform::default(),
        Head => Transform::from_xyz(0., 0.65, 0.012),
        Jaw => Transform::from_xyz(0., -0.116, -0.09),
        WristL | WristR => {
            Transform::from_xyz(if j == WristL { -0.045 } else { 0.045 }, -0.13, -0.30)
        }
        FingersL | FingersR => Transform::from_xyz(0., 0.007, -0.017),
        ArmL | ArmR => Transform::from_xyz(if j == ArmL { -0.13 } else { 0.13 }, 0.36, 0.008),
        EarL | EarR => {
            Transform::from_xyz(if j == EarL { -0.145 * w } else { 0.145 * w }, 0.10, 0.052)
        }
        EyeL | EyeR => Transform::from_xyz(
            if j == EyeL { -0.066 * w } else { 0.066 * w },
            0.043,
            -0.166,
        ),
        LidL | LidR | LowerLidL | LowerLidR => Transform::from_xyz(
            if matches!(j, LidL | LowerLidL) {
                -0.066 * w
            } else {
                0.066 * w
            },
            0.037,
            -0.126,
        ),
        BrowL | BrowR => Transform::from_xyz(
            if j == BrowL { -0.066 * w } else { 0.066 * w },
            0.105,
            -0.104,
        ),
    }
}
pub fn spawn(c:&mut Commands,parent:Entity,pieces:&[Piece],breed:usize,owner:usize,portrait:bool)->Entity {spawn_layer(c,parent,pieces,breed,owner,portrait,if portrait{PORTRAIT_LAYER}else{DOG_LAYER})}
pub fn spawn_layer(
    c: &mut Commands,
    parent: Entity,
    pieces: &[Piece],
    breed: usize,
    owner: usize,
    portrait: bool,
    layer:usize,
) -> Entity {
    use Joint::*;

    let root = c
        .spawn((
            DogRoot,
            Transform::from_xyz(0., 0.34, -2.40),
            Visibility::Inherited,
        ))
        .id();
    c.entity(parent).add_child(root);
    let mut nodes = BTreeMap::new();
    nodes.insert(Body, root);
    for j in [
        Chest, Head, Jaw, Tongue, EarL, EarR, EyeL, EyeR, LidL, LidR, LowerLidL, LowerLidR, BrowL,
        BrowR, ArmL, ArmR, WristL, WristR, FingersL, FingersR,
    ] {
        let t = rest(j, breed);
        let node = c
            .spawn((
                t,
                Visibility::Inherited,
                Bone {
                    owner,
                    portrait,
                    joint: j,
                    rest: t,
                },
            ))
            .id();
        let p = match j {
            WristL => nodes[&ArmL],
            WristR => nodes[&ArmR],
            FingersL => nodes[&WristL],
            FingersR => nodes[&WristR],
            Chest => root,
            Head | ArmL | ArmR => nodes[&Chest],
            Tongue => nodes[&Jaw],
            _ => nodes[&Head],
        };
        c.entity(p).add_child(node);
        nodes.insert(j, node);
    }
    for p in pieces {
        let e = c
            .spawn((
                Mesh3d(p.mesh.clone()),
                MeshMaterial3d(p.material.clone()),
                Transform::default(),
                RenderLayers::layer(layer),
            ))
            .id();
        if !portrait {
            c.entity(e).insert(bevy::light::NotShadowCaster);
        }
        c.entity(nodes[&p.joint]).add_child(e);
    }
    root
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detailed_rigs_have_finite_meshes_for_all_ten_breeds() {
        for (breed, equipment) in (0..10).flat_map(|b| (0..3).map(move |e| (b, e))) {
            let (mut meshes, mut mats, mut images) =
                (Assets::default(), Assets::default(), Assets::default());
            let p = build(
                &mut meshes,
                &mut mats,
                &mut images,
                &Appearance {
                    breed,
                    equipment,
                    ..default()
                },
            );
            assert!(p.len() > 25);
            for p in p {
                let m = meshes.get(&p.mesh).unwrap();
                let n = m.count_vertices();
                assert!(m.attributes().all(|(_, a)| a.len() == n));
                for attr in [Mesh::ATTRIBUTE_POSITION, Mesh::ATTRIBUTE_NORMAL] {
                    let Some(bevy::mesh::VertexAttributeValues::Float32x3(v)) = m.attribute(attr)
                    else {
                        panic!()
                    };
                    assert!(
                        v.iter().flatten().all(|f| f.is_finite()),
                        "breed={breed} equipment={equipment} joint={:?} attribute={attr:?}",
                        p.joint
                    );
                }
                assert!(m.indices().unwrap().iter().all(|i| i < n));
            }
        }
    }
}
