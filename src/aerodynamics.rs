//! Transparent, tunable swept delta/canard model. Coefficients are not Saab flight-test data.
use bevy::math::DVec3;
use std::f64::consts::PI;
pub const SPAN: f64 = 8.4;
pub const AREA: f64 = 30.0;
pub const CHORD: f64 = 3.5;
const AR: f64 = SPAN * SPAN / AREA;
#[derive(Clone, Copy, Debug)]
pub struct FlowState {
    pub lag_alpha: f64,
    pub lag_beta: f64,
    pub vortex_lift: f64,
    pub attached: f64,
}
impl Default for FlowState {
    fn default() -> Self {
        Self {
            lag_alpha: 0.08,
            lag_beta: 0.0,
            vortex_lift: 0.0,
            attached: 1.0,
        }
    }
}
#[derive(Clone, Copy)]
pub struct Conditions {
    pub alpha: f64,
    pub beta: f64,
    pub mach: f64,
    pub speed: f64,
    pub density: f64,
    pub temperature: f64,
    pub flaps: f64,
    pub gear: bool,
    pub airbrake: bool,
    pub height: f64,
    pub rates: DVec3,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Coefficients {
    pub lift: f64,
    pub drag: f64,
    pub side: f64,
    pub pitch: f64,
    pub yaw: f64,
    pub roll: f64,
    pub parasite: f64,
    pub induced: f64,
    pub wave: f64,
    pub control_drag: f64,
    pub control_scale: f64,
    pub attached: f64,
}
fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
fn finite_wing_slope(mach: f64) -> f64 {
    finite_wing_slope_for(mach,AR,40_f64.to_radians())
}
fn finite_wing_slope_for(mach:f64,ar:f64,sweep:f64)->f64 {
    // Bounded subsonic finite-wing correction, smoothly joined to a supersonic slope.
    let beta = (1.0 - mach.min(0.85).powi(2)).sqrt();
    let sub =
        2.0 * PI * ar / (2.0 + (4.0 + ar * ar * (beta * beta + sweep.tan().powi(2))).sqrt()) + 0.65;
    let sup = (4.0 / (mach.max(1.15).powi(2) - 1.0).sqrt()).min(3.7);
    let t = smooth(0.85, 1.30, mach);
    sub * (1.0 - t) + sup * t
}
pub fn induced_drag(lift: f64, attached: f64, beta: f64, height: f64) -> f64 {
    induced_for(lift,attached,beta,height,SPAN,AR)
}
pub fn raptor_induced(lift:f64,attached:f64,beta:f64,height:f64)->f64 {
    induced_for(lift,attached,beta,height,13.56,13.56*13.56/78.)
}
fn induced_for(lift:f64,attached:f64,beta:f64,height:f64,span:f64,ar:f64)->f64 {
    let efficiency = (0.82 - 0.18 * (1.0 - attached) - 0.08 * beta.abs()).clamp(0.45, 0.86);
    let ground = (16.0 * height.max(0.0) / span).powi(2);
    let ground_factor = (ground / (1.0 + ground)).clamp(0.22, 1.0);
    lift * lift / (PI * ar * efficiency) * ground_factor
}
pub fn evaluate(c: Conditions, state: &mut FlowState, dt: f64) -> Coefficients {
    evaluate_geometry(c,state,dt,false)
}
pub fn evaluate_raptor(c:Conditions,state:&mut FlowState,dt:f64)->Coefficients {
    evaluate_geometry(c,state,dt,true)
}
fn evaluate_geometry(c:Conditions,state:&mut FlowState,dt:f64,raptor:bool)->Coefficients {
    let chord=if raptor{78./13.56}else{CHORD};let span=if raptor{13.56}else{SPAN};
    let v = c.speed.max(1.0);
    let tau = (chord / v * 1.2).clamp(0.025, 0.20);
    state.lag_alpha += (c.alpha - state.lag_alpha) * (1.0 - (-dt / tau).exp());
    let beta_rate = (c.beta - state.lag_beta) / (2.0 * tau);
    state.lag_beta += (c.beta - state.lag_beta) * (1.0 - (-dt / (2.0 * tau)).exp());
    let critical = ((if raptor{0.42}else{0.31}) + c.flaps * 0.025 - 0.055 * smooth(0.75, 1.30, c.mach)).max(0.24);
    // Reattachment needs a lower incidence than separation: a simple hysteresis model.
    let threshold = critical - 0.045 * (1.0 - state.attached);
    let target = 1.0 - smooth(threshold - 0.035, threshold + 0.07, state.lag_alpha.abs());
    let separation_tau = if target < state.attached {
        tau * 1.4
    } else {
        tau * 3.5
    };
    state.attached += (target - state.attached) * (1.0 - (-dt / separation_tau).exp());
    state.attached = state.attached.clamp(0.0, 1.0);
    let a = c.alpha;
    // A bounded transient vortex contribution during rapid nose-up motion.
    // Its convective relaxation distinguishes pitching through stall from a
    // steady polar; the constants are game-model assumptions, not NASA fits.
    let pitch_incidence_rate = ((c.alpha - state.lag_alpha) / tau).clamp(-4.0, 4.0);
    let onset = smooth(0.12, 0.28, a.abs()) * (1.0 - smooth(0.5, 0.9, a.abs()));
    let vortex_target = (pitch_incidence_rate * chord / v * 0.65).clamp(-0.18, 0.18) * onset;
    state.vortex_lift += (vortex_target - state.vortex_lift) * (1.0 - (-dt / (tau * 2.5)).exp());
    let pitch_rate = c.rates.x * chord / (2.0 * v);
    let yaw_rate = c.rates.y * span / (2.0 * v);
    let roll_rate = c.rates.z * span / (2.0 * v);
    let slope = if raptor{finite_wing_slope_for(c.mach,span*span/78.,42_f64.to_radians())}else{finite_wing_slope(c.mach)};
    let vortex =
        2.2 * a.sin().powi(2) * a.cos() * a.signum() * (1.0 - 0.6 * smooth(0.7, 1.5, c.mach));
    let attached_lift = 0.16 + c.flaps * 0.35 + slope * a + vortex + 1.1 * pitch_rate;
    let separated_lift = 0.95 * (2.0 * a).sin();
    let lift = (state.attached * attached_lift
        + (1.0 - state.attached) * separated_lift
        + state.vortex_lift)
        * c.beta.cos().powi(2);
    // Sutherland viscosity and turbulent skin friction give Reynolds-dependent parasite drag.
    let t = c.temperature.max(120.0);
    let viscosity = 1.716e-5 * (t / 273.15).powf(1.5) * (273.15 + 110.4) / (t + 110.4);
    let reynolds = (c.density * v * chord / viscosity).max(10000.0);
    let cf = 0.455 / (reynolds.log10().powf(2.58) * (1.0 + 0.144 * c.mach * c.mach).powf(0.65));
    let parasite = 0.010
        + cf * 3.8
        + if c.gear { 0.022 } else { 0.0 }
        + c.flaps * 0.030
        + if c.airbrake { 0.13 } else { 0.0 };
    let induced = if raptor{raptor_induced(lift,state.attached,c.beta,c.height)}else{induced_drag(lift, state.attached, c.beta, c.height)};
    let wave = 0.028 * (-((c.mach - 1.06) / 0.17).powi(2)).exp() + 0.022 * smooth(1.0, 1.5, c.mach);
    let separation_drag = (1.0 - state.attached) * 1.25 * a.sin().powi(2);
    let reverse_drag = 0.18 * (1.0 - a.cos()).max(0.0);
    let drag =
        (parasite + induced + wave + separation_drag + reverse_drag + 0.22 * c.beta.sin().powi(2))
            .max(0.0);
    let control_scale = (0.30 + 0.70 * state.attached) * (1.0 - 0.25 * smooth(0.8, 1.4, c.mach));
    // Mach tuck and separated-flow pitch break; derivative signs match this game's body axes.
    let pitch = 0.008
        - 0.10 * a
        - 3.0 * pitch_rate
        - 0.015 * c.flaps
        - 0.035 * smooth(0.80, 1.15, c.mach) * a
        - 0.08 * (1.0 - state.attached) * a.signum()
        - 0.08 * state.vortex_lift;
    let unsteady_beta = (beta_rate * span / (2.0 * v)).clamp(-0.2, 0.2);
    let yaw = -0.12 * c.beta - 0.80 * yaw_rate - 0.12 * unsteady_beta;
    let roll = -0.08 * c.beta - 0.50 * roll_rate - 0.08 * unsteady_beta;
    Coefficients {
        lift,
        drag,
        side: -0.65 * c.beta - 0.15 * yaw_rate,
        pitch,
        yaw,
        roll,
        parasite,
        induced,
        wave,
        control_drag: 0.0,
        control_scale,
        attached: state.attached,
    }
}
/// Lift and side force are perpendicular to air velocity; drag alone removes kinetic energy.
pub fn force(body_right: DVec3, air: DVec3, q_area: f64, c: Coefficients) -> DVec3 {
    let flow = air.normalize_or_zero();
    let lift_axis = body_right.cross(flow).normalize_or_zero();
    let side_axis = flow.cross(lift_axis).normalize_or_zero();
    q_area * (lift_axis * c.lift + side_axis * c.side - flow * c.drag)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transient_vortex_relaxes_when_incidence_stops_changing() {
        let mut s = FlowState::default();
        evaluate(condition(0.3), &mut s, 0.01);
        assert!(s.vortex_lift > 0.0);
        for _ in 0..1000 {
            evaluate(condition(0.3), &mut s, 0.01);
        }
        assert!(s.vortex_lift.abs() < 1e-8);
    }
    #[test]
    fn induced_drag_uses_total_lift_and_is_nonnegative() {
        assert!(induced_drag(1.0, 1.0, 0.0, 1000.0) > induced_drag(0.5, 1.0, 0.0, 1000.0) * 3.99);
        assert_eq!(induced_drag(0.0, 1.0, 0.0, 1000.0), 0.0);
        assert_eq!(
            induced_drag(-1.0, 1.0, 0.0, 1000.0),
            induced_drag(1.0, 1.0, 0.0, 1000.0)
        );
    }
    fn condition(a: f64) -> Conditions {
        Conditions {
            alpha: a,
            beta: 0.0,
            mach: 0.25,
            speed: 85.0,
            density: 1.2,
            temperature: 285.0,
            flaps: 0.0,
            gear: false,
            airbrake: false,
            height: 1000.0,
            rates: DVec3::ZERO,
        }
    }
    fn steady(c: Conditions) -> Coefficients {
        let mut s = FlowState {
            lag_alpha: c.alpha,
            attached: 1.0,
            ..Default::default()
        };
        let mut out = Coefficients::default();
        for _ in 0..1000 {
            out = evaluate(c, &mut s, 0.01);
        }
        out
    }
    #[test]
    fn lift_grows_before_stall() {
        assert!(steady(condition(0.15)).lift > steady(condition(0.05)).lift);
    }
    #[test]
    fn separation_and_control_loss() {
        let low = steady(condition(0.1));
        let high = steady(condition(0.8));
        assert!(high.attached < 0.01);
        assert!(high.control_scale < low.control_scale);
        assert!(high.drag > low.drag);
    }
    #[test]
    fn ground_reduces_induced_drag() {
        let high = steady(condition(0.10));
        let mut c = condition(0.10);
        c.height = 0.5;
        let low = steady(c);
        assert!(low.induced < high.induced);
    }
    #[test]
    fn continuous_stall_curve() {
        let a = steady(condition(0.3099));
        let b = steady(condition(0.3101));
        assert!((a.lift - b.lift).abs() < 0.02);
    }
    #[test]
    fn mach_sweep_stays_finite() {
        for i in 0..301 {
            let mut c = condition(0.15);
            c.mach = i as f64 / 100.0;
            let o = steady(c);
            assert!(o.lift.is_finite() && o.drag.is_finite() && o.drag > 0.0);
        }
    }
    #[test]
    fn aerodynamic_power_is_drag_only() {
        let air = DVec3::new(12.0, -8.0, -100.0);
        let c = Coefficients {
            lift: 1.1,
            side: -0.4,
            drag: 0.12,
            ..Default::default()
        };
        let f = force(DVec3::X, air, 1000.0, c);
        assert!((f.dot(air) + 1000.0 * 0.12 * air.length()).abs() < 1e-8);
    }
    #[test]
    fn rate_damping_opposes_motion() {
        let mut c = condition(0.0);
        c.rates = DVec3::new(0.2, 0.2, 0.2);
        let o = steady(c);
        assert!(o.pitch < 0.0 && o.yaw < 0.0 && o.roll < 0.0);
    }
    #[test]
    fn separation_has_memory() {
        let mut s = FlowState::default();
        for _ in 0..100 {
            evaluate(condition(0.65), &mut s, 0.01);
        }
        evaluate(condition(0.0), &mut s, 0.001);
        assert!(s.attached < 0.1 && s.lag_alpha > 0.4);
        for _ in 0..100 {
            evaluate(condition(0.0), &mut s, 0.01);
        }
        assert!(s.attached > 0.99);
    }
}
