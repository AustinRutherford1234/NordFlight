//! Gameplay propulsion and a steady level-flight envelope diagnostic.
//! The wet-thrust/intake values are tuned to the requested Mach 2 at 60,000 ft.
use crate::{aerodynamics as aero, control_surfaces as surfaces, physics::*};
use bevy::math::{DQuat, DVec3};

pub const DRY_THRUST: f64 = 50_400.0;
pub const AFTERBURNER_THRUST: f64 = 60_000.0;
pub const CRUISE_ALTITUDE: f64 = 60_000.0 * 0.3048;
const HIGH_MACH_LOSS: f64 = 0.25;

pub fn jet_output(spool: f64, density: f64, mach: f64) -> (f64, f64) {
    JetConditions::new(density, mach).output(spool)
}
/// Share atmospheric/intake terms across a pilot's ten throttle-search probes.
pub struct JetConditions {
    lapse: f64,
}
impl JetConditions {
    pub fn new(density: f64, mach: f64) -> Self {
        let t = ((mach - 1.4) / 1.0).clamp(0.0, 1.0);
        let recovery = 1.0 - HIGH_MACH_LOSS * t * t * (3.0 - 2.0 * t);
        let lapse = (density / 1.225).clamp(0.0, 1.0).powf(0.65)
            * (1.0 + 0.16 * mach.max(0.0)).min(1.3)
            * recovery;
        Self { lapse }
    }
    pub fn output(&self, spool: f64) -> (f64, f64) {
        let power = spool.clamp(0.0, 1.0);
        let dry = DRY_THRUST * power.min(0.88) / 0.88;
        let burner = AFTERBURNER_THRUST * ((power - 0.88) / 0.12).clamp(0.0, 1.0);
        ((dry + burner) * self.lapse, burner * self.lapse)
    }
}

#[derive(Clone, Copy)]
struct Trim {
    alpha: f64,
    actuator: f64,
    drag: f64,
    thrust: f64,
}
fn trim(altitude: f64, mach: f64, mass: f64) -> Trim {
    let (temperature, _, density, sound) = atmosphere(altitude);
    let speed = sound * mach;
    let q_area = 0.5 * density * speed * speed * WING_AREA;
    let thrust = jet_output(1.0, density, mach).0;
    let weight = mass * (MU / (RADIUS + altitude).powi(2) - speed.powi(2) / (RADIUS + altitude));
    let loads = |alpha: f64| {
        let air = DVec3::new(0.0, -speed * alpha.sin(), -speed * alpha.cos());
        let mut state = aero::FlowState {
            lag_alpha: alpha,
            lag_beta: 0.0,
            vortex_lift: 0.0,
            attached: 1.0,
        };
        let coeff = aero::evaluate(
            aero::Conditions {
                alpha,
                beta: 0.0,
                mach,
                speed,
                density,
                temperature,
                flaps: 0.0,
                gear: false,
                airbrake: false,
                height: altitude,
                rates: DVec3::ZERO,
            },
            &mut state,
            1.0,
        );
        let panel = |actuator: f64| {
            surfaces::evaluate_flight(
                air,
                DVec3::ZERO,
                density,
                coeff.control_scale,
                DVec3::X * actuator,
                0.0,
                false,
                mach,
            )
        };
        let natural = q_area * 3.5 * coeff.pitch;
        let (mut low, mut high) = (-1.0, 1.0);
        for _ in 0..28 {
            let middle = (low + high) * 0.5;
            if natural + panel(middle).moment.x > 0.0 {
                high = middle;
            } else {
                low = middle;
            }
        }
        let actuator = (low + high) * 0.5;
        let panels = panel(actuator);
        let flow = air.normalize();
        let lift_axis = DVec3::X.cross(flow).normalize();
        let lift = q_area * coeff.lift + panels.force.dot(lift_axis);
        let cl = lift / q_area;
        let drag = q_area
            * (coeff.drag + aero::induced_drag(cl, coeff.attached, 0.0, altitude) - coeff.induced)
            - panels.force.dot(flow);
        (lift + thrust * alpha.sin(), drag, actuator)
    };
    let (mut low, mut high) = (-0.10, 0.25);
    for _ in 0..32 {
        let middle = (low + high) * 0.5;
        if loads(middle).0 > weight {
            high = middle;
        } else {
            low = middle;
        }
    }
    let alpha = (low + high) * 0.5;
    let (_, drag, actuator) = loads(alpha);
    Trim {
        alpha,
        actuator,
        drag,
        thrust,
    }
}
fn terminal_mach(altitude: f64, mass: f64) -> f64 {
    let (mut low, mut high) = (1.3, 2.5);
    for _ in 0..32 {
        let middle = (low + high) * 0.5;
        let t = trim(altitude, middle, mass);
        if t.thrust * t.alpha.cos() > t.drag {
            low = middle;
        } else {
            high = middle;
        }
    }
    (low + high) * 0.5
}
pub fn study() {
    let mut rows = Vec::new();
    for altitude in [2500.0, CRUISE_ALTITUDE] {
        for mach in [0.8, 0.9, 1.0, 1.06, 1.2, 1.8, 1.9, 2.0, 2.1, 2.2] {
            let t = trim(altitude, mach, 8300.0);
            let (_, _, density, sound) = atmosphere(altitude);
            let legacy_thrust =
                80_500.0 * (density / 1.225).powf(0.65) * (1.0 + 0.16 * mach).min(1.3);
            let legacy_drag = t.drag
                + 0.5
                    * density
                    * (sound * mach).powi(2)
                    * WING_AREA
                    * (0.085 - 0.028)
                    * (-((mach - 1.06) / 0.17).powi(2)).exp();
            rows.push(
                serde_json::json!({"altitude_m":altitude,"mach":mach,"thrust_n":t.thrust,
                "drag_n":t.drag,"excess_thrust_n":t.thrust*t.alpha.cos()-t.drag,
                "trim_alpha_deg":t.alpha.to_degrees(),"pitch_actuator":t.actuator,
                "legacy_thrust_n":legacy_thrust,"legacy_drag_estimate_n":legacy_drag}),
            );
        }
    }
    let mut flights = Vec::new();
    for (altitude, initial_mach, steps) in [
        (CRUISE_ALTITUDE, 1.8, 43200),
        (CRUISE_ALTITUDE, 2.1, 43200),
        (2500.0, 0.85, 21600),
    ] {
        let mut f = Flight::approach();
        let mut e = Environment::default();
        e.wind_enabled = false;
        f.position = surface_point(0.0, 0.0, altitude);
        f.fuel = 1500.0;
        f.infinite_fuel = true;
        f.gear = false;
        f.gear_extension = 0.0;
        f.gear_door = 0.0;
        f.flaps = 0.0;
        f.throttle = 1.0;
        f.spool = 1.0;
        let (_, _, _, sound) = atmosphere(altitude);
        f.velocity = -DVec3::Z * (initial_mach * sound);
        let mut pitch_trim = trim(altitude, initial_mach, f.mass());
        f.orientation = DQuat::from_rotation_x(pitch_trim.alpha);
        f.surface = DVec3::X * pitch_trim.actuator;
        f.flow.lag_alpha = pitch_trim.alpha;
        let mut min_alt = f.altitude();
        let mut max_alt = f.altitude();
        for step in 0..steps {
            if step % 120 == 0 {
                pitch_trim = trim(altitude, f.velocity.length() / sound, f.mass());
            }
            let path = ((altitude - f.altitude()) * 0.00018 - f.vertical_speed() * 0.012)
                .clamp(-0.06, 0.06);
            f.controls = DVec3::new(
                ((pitch_trim.alpha + path - f.pitch()) * 3.0).clamp(-0.5, 0.5),
                -f.beta * 2.0,
                (f.roll() * -2.0).clamp(-0.5, 0.5),
            );
            e.advance(1.0 / 120.0);
            simulate(&mut f, &e, 1.0 / 120.0);
            min_alt = min_alt.min(f.altitude());
            max_alt = max_alt.max(f.altitude());
            if f.crashed {
                break;
            }
        }
        flights.push(
            serde_json::json!({"target_altitude_m":altitude,"initial_mach":initial_mach,"final_mach":f.mach,
            "final_altitude_m":f.altitude(),"min_altitude_m":min_alt,"max_altitude_m":max_alt,
            "elapsed_s":f.elapsed,"crashed":f.crashed,"integrity":f.integrity}),
        );
    }
    let terminal = terminal_mach(CRUISE_ALTITUDE, 8300.0);
    let report = serde_json::json!({"description":"Game envelope tuning; clean aircraft, 1500 kg fuel, full afterburner, still air",
        "reference_altitude_m":CRUISE_ALTITUDE,"steady_terminal_mach":terminal,"balance":rows,"controlled_flights":flights});
    std::fs::write(
        "tests/jet-performance.json",
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_intake_terms_preserve_thrust_at_throttle_and_mach_boundaries() {
        for density in [0., 0.01, 0.3, 1.225, 1.8] {
            for mach in [-0.2, 0., 0.8, 1.4, 2., 2.4, 3.] {
                let engine = JetConditions::new(density, mach);
                for spool in [-0.1_f64, 0., 0.2, 0.88, 0.9, 1., 1.1] {
                    let power = spool.clamp(0., 1.);
                    let dry = DRY_THRUST * power.min(0.88) / 0.88;
                    let burner = AFTERBURNER_THRUST * ((power - 0.88) / 0.12).clamp(0., 1.);
                    let t = ((mach - 1.4_f64) / 1.).clamp(0., 1.);
                    let recovery = 1. - HIGH_MACH_LOSS * t * t * (3. - 2. * t);
                    let lapse = (density / 1.225_f64).clamp(0., 1.).powf(0.65)
                        * (1. + 0.16 * mach.max(0.)).min(1.3)
                        * recovery;
                    let expected = ((dry + burner) * lapse, burner * lapse);
                    let actual = engine.output(spool);
                    assert_eq!(actual.0.to_bits(), expected.0.to_bits());
                    assert_eq!(actual.1.to_bits(), expected.1.to_bits());
                }
            }
        }
    }
    #[test]
    fn full_afterburner_crosses_transonic_drag_peak() {
        for mach in [0.9, 1.0, 1.06, 1.15] {
            let t = trim(2500.0, mach, 8300.0);
            assert!(
                t.thrust * t.alpha.cos() > t.drag,
                "Mach {mach}: T {} D {}",
                t.thrust,
                t.drag
            );
        }
    }
    #[test]
    fn clean_level_aircraft_balances_near_mach_two_at_sixty_thousand_feet() {
        let maximum = terminal_mach(CRUISE_ALTITUDE, 8300.0);
        assert!((maximum - 2.0).abs() < 0.01, "terminal Mach {maximum}");
        let below = trim(CRUISE_ALTITUDE, 1.9, 8300.0);
        let above = trim(CRUISE_ALTITUDE, 2.1, 8300.0);
        assert!(below.thrust * below.alpha.cos() > below.drag);
        assert!(above.thrust * above.alpha.cos() < above.drag);
    }
    #[test]
    fn throttle_output_is_monotonic_and_vacuum_has_no_jet_thrust() {
        let mut previous = 0.0;
        for i in 0..=100 {
            let output = jet_output(i as f64 / 100.0, 1.225, 0.7).0;
            assert!(output >= previous);
            previous = output;
        }
        assert_eq!(jet_output(1.0, 0.0, 2.0), (0.0, 0.0));
        assert_eq!(jet_output(0.88, 1.225, 0.0).0, DRY_THRUST);
    }
}
