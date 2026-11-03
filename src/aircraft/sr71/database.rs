//! Bounded trilinear Mach/incidence/sideslip database, plus control-deflection grid.
//! Vertices are authored from public derivative trends and documented estimates;
//! tabulation does not turn estimated high-alpha values into measured evidence.
use super::{aero, config::*, math::*};
use crate::aerodynamics::Conditions;
use std::sync::OnceLock;
pub const MACH_AXIS: [f64; 11] = [0., 0.4, 0.6, 0.9, 1., 1.1, 1.6, 2., 2.5, 3., 3.5];
pub const ALPHA_DEG: [f64; 25] = [
    -180., -150., -120., -90., -60., -45., -30., -22., -18., -14., -10., -5., 0., 5., 10., 14.,
    18., 22., 30., 45., 60., 90., 120., 150., 180.,
];
pub const BETA_DEG: [f64; 7] = [-45., -25., -10., 0., 10., 25., 45.];
pub const DELTA_DEG: [f64; 7] = [-35., -25., -10., 0., 10., 20., 35.];
#[derive(Clone, Copy, Debug, Default)]
pub struct Polar {
    pub lift: f64,
    pub drag: f64,
    pub side: f64,
    pub pitch: f64,
    pub yaw: f64,
    pub roll: f64,
    pub attached: f64,
    pub profile_drag: f64,
    pub wave_drag: f64,
}
impl Polar {
    fn array(self) -> [f64; 9] {
        [
            self.lift,
            self.drag,
            self.side,
            self.pitch,
            self.yaw,
            self.roll,
            self.attached,
            self.profile_drag,
            self.wave_drag,
        ]
    }
    fn from(a: [f64; 9]) -> Self {
        Self {
            lift: a[0],
            drag: a[1],
            side: a[2],
            pitch: a[3],
            yaw: a[4],
            roll: a[5],
            attached: a[6],
            profile_drag: a[7],
            wave_drag: a[8],
        }
    }
}
fn vertex(m: f64, a: f64, b: f64) -> Polar {
    let c = Conditions {
        alpha: a,
        beta: b,
        mach: m,
        speed: 300.,
        density: 0.4,
        temperature: 230.,
        flaps: 0.,
        gear: false,
        airbrake: false,
        height: 10000.,
        rates: bevy::math::DVec3::ZERO,
    };
    let p = aero::steady(c);
    let incidence = a - INCIDENCE;
    let attached = p.attached;
    let normal_plate = 1.85 * incidence.sin() * incidence.sin().abs();
    let old_plate = 1.5 * (2. * incidence).sin() * incidence.cos();
    let lift = (p.lift - (1. - attached) * old_plate
        + (1. - attached) * normal_plate * incidence.cos())
        * b.cos().powi(2);
    Polar {
        lift,
        drag: p.parasite
            + p.wave
            + (1. - attached) * normal_plate * incidence.sin()
            + AERO.incidence_drag * (1. - incidence.cos()).max(0.),
        side: p.side * (1. - 0.35 * incidence.sin().powi(2)),
        pitch: p.pitch,
        yaw: p.yaw * (1. - 0.5 * incidence.sin().powi(2)),
        roll: p.roll,
        attached,
        profile_drag: p.parasite + AERO.incidence_drag * (1. - incidence.cos()).max(0.),
        wave_drag: p.wave,
    }
}
fn database() -> &'static Vec<Polar> {
    static DB: OnceLock<Vec<Polar>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut v = Vec::with_capacity(MACH_AXIS.len() * ALPHA_DEG.len() * BETA_DEG.len());
        for m in MACH_AXIS {
            for a in ALPHA_DEG {
                for b in BETA_DEG {
                    v.push(vertex(m, a.to_radians(), b.to_radians()));
                }
            }
        }
        v
    })
}
pub fn bracket(axis: &[f64], x: f64) -> (usize, f64) {
    let x = x.clamp(axis[0], axis[axis.len() - 1]);
    let i = axis
        .windows(2)
        .position(|p| x <= p[1])
        .unwrap_or(axis.len() - 2);
    (i, (x - axis[i]) / (axis[i + 1] - axis[i]))
}
pub fn lookup(m: f64, alpha: f64, beta: f64) -> Polar {
    let a = alpha.sin().atan2(alpha.cos()) * DEG;
    let (mi, mt) = bracket(&MACH_AXIS, m);
    let (ai, at) = bracket(&ALPHA_DEG, a);
    let (bi, bt) = bracket(&BETA_DEG, beta * DEG);
    let mut out = [0.; 9];
    for dm in 0..2 {
        for da in 0..2 {
            for db in 0..2 {
                let w = if dm == 0 { 1. - mt } else { mt }
                    * if da == 0 { 1. - at } else { at }
                    * if db == 0 { 1. - bt } else { bt };
                let p = database()
                    [((mi + dm) * ALPHA_DEG.len() + ai + da) * BETA_DEG.len() + bi + db]
                    .array();
                for k in 0..9 {
                    out[k] += w * p[k];
                }
            }
        }
    }
    Polar::from(out)
}
/// Nonlinear control effectiveness versus deflection and incidence. Estimated grid.
pub fn control_factor(m: f64, alpha: f64, delta: f64) -> f64 {
    let shape = [0.78, 0.90, 0.98, 1., 0.98, 0.94, 0.78];
    let deflection = table(&DELTA_DEG, &shape, delta * DEG);
    deflection * (1. - 0.45 * smooth(0.20, 0.65, alpha.abs())) * (1. - 0.12 * smooth(0.9, 3., m))
}
pub fn register() -> serde_json::Value {
    serde_json::json!({"schema":1,"axes":{"mach":MACH_AXIS,"alpha_deg":ALPHA_DEG,"beta_deg":BETA_DEG,"control_deg":DELTA_DEG},"vertices":MACH_AXIS.len()*ALPHA_DEG.len()*BETA_DEG.len(),"source":"NASA/TP-2002-210718 baseline small-disturbance trends; nonlinear vertices, coupling and control saturation are engineering estimates","interpolation":"bounded trilinear; value-continuous, not derivative-continuous","reynolds":"analytic skin friction/local lift correction, not another measured table dimension"})
}

/// Offline export preserves the authored/estimated label. No runtime filesystem work.
pub fn export(path: &std::path::Path) -> std::io::Result<()> {
    use std::io::Write;
    let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(
        out,
        "mach,alpha_deg,beta_deg,cl,cd_profile_separated_wave,cy,cm,cn,cl_roll,attachment,source_kind"
    )?;
    for m in MACH_AXIS {
        for a in ALPHA_DEG {
            for b in BETA_DEG {
                let p = lookup(m, a.to_radians(), b.to_radians());
                writeln!(
                    out,
                    "{m},{a},{b},{},{},{},{},{},{},{},authored_estimate",
                    p.lift, p.drag, p.side, p.pitch, p.yaw, p.roll, p.attached
                )?;
            }
        }
    }
    out.flush()
}
