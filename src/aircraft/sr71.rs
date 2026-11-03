//! SR-71 actuator state and dedicated aerodynamic plant.
pub mod flight;
use crate::physics::*;
use bevy::math::DVec3;
#[derive(Clone, Default)]
pub struct State {
    pub canopy_command: bool,
    pub canopy: f64,
    pub elevons: [f64; 4],
    pub fins: [f64; 2],
    pub spike: f64,
    pub nozzles: [f64; 2],
    pub diagnostics: flight::diagnostics::State,
    pub airframe:flight::plant::State,
}
fn move_to(value: &mut f64, target: f64, rate: f64, dt: f64) {
    *value += (target - *value).clamp(-rate * dt, rate * dt);
}
pub fn advance(f: &mut Flight, dt: f64, mach: f64) {
    if dt <= 0. || !dt.is_finite() {
        return;
    }
    let authority = ((f.damage.hydraulics - flight::config::CONTROL.hydraulic_floor)
        / (1. - flight::config::CONTROL.hydraulic_floor))
        .clamp(0., 1.);
    let commands = f.surface;
    let state = &mut f.systems.sr71;
    let targets = flight::targets(commands);
    for i in 0..4 {
        move_to(
            &mut state.elevons[i],
            targets[i],
            flight::config::CONTROL.elevon_rate_rad_s * authority,
            dt,
        );
    }
    for i in 0..2 {
        move_to(
            &mut state.fins[i],
            commands.y.clamp(-1., 1.) * flight::config::CONTROL.rudder_limit_rad,
            flight::config::CONTROL.rudder_rate_rad_s * authority,
            dt,
        );
    }
    for (i, slot) in [0, 4, 2, 6].into_iter().enumerate() {
        match f.damage.surface_state[slot] {
            crate::combat::damage::SurfaceState::Jammed(a) => {
                state.elevons[i] = a.clamp(
                    -flight::config::CONTROL.elevon_up_rad,
                    if i < 2 {
                        flight::config::CONTROL.inner_down_rad
                    } else {
                        flight::config::CONTROL.outer_down_rad
                    },
                )
            }
            crate::combat::damage::SurfaceState::Detached => state.elevons[i] = 0.,
            _ => {}
        }
    }
    for (i, slot) in [3, 7].into_iter().enumerate() {
        match f.damage.surface_state[slot] {
            crate::combat::damage::SurfaceState::Jammed(a) => {
                state.fins[i] = a.clamp(
                    -flight::config::CONTROL.rudder_limit_rad,
                    flight::config::CONTROL.rudder_limit_rad,
                )
            }
            crate::combat::damage::SurfaceState::Detached => state.fins[i] = 0.,
            _ => {}
        }
    }
    move_to(
        &mut state.canopy,
        if state.canopy_command { 1. } else { 0. },
        flight::config::REGIMES.canopy_rate_s * authority,
        dt,
    );
    // Representative spike schedule shared with the installed inlet-loss model.
    let target = flight::config::ENGINE.spike_travel_m
        * ((mach - flight::config::ENGINE.spike_start_mach)
            / flight::config::ENGINE.spike_mach_interval)
            .clamp(0., 1.);
    move_to(
        &mut state.spike,
        target,
        flight::config::REGIMES.spike_rate_m_s * authority,
        dt,
    );
    for i in 0..2 {
        let target = if f.engine && f.systems.engine_on[i] {
            f.spool.clamp(0., 1.)
        } else {
            0.
        };
        move_to(
            &mut state.nozzles[i],
            target,
            flight::config::REGIMES.nozzle_rate_s * authority,
            dt,
        );
    }
}
pub fn smooth(x: f64) -> f64 {
    let x = x.clamp(0., 1.);
    x * x * (3. - 2. * x)
}
pub fn gear_rotation(extension: f64, index: usize) -> bevy::math::DQuat {
    let angle = smooth(1. - extension) * std::f64::consts::FRAC_PI_2;
    if index == 0 {
        bevy::math::DQuat::from_rotation_x(angle)
    } else {
        bevy::math::DQuat::from_rotation_z(if index == 1 { angle } else { -angle })
    }
}
pub fn mount(index: usize) -> DVec3 {
    match index {
        0 => DVec3::new(0., 0., -5.52),
        1 => DVec3::new(-2.56, 0.21, 6.27),
        _ => DVec3::new(2.56, 0.21, 6.27),
    }
}
pub fn wheel(index: usize) -> DVec3 {
    match index {
        0 => DVec3::new(0., -1.714, -5.523),
        1 => DVec3::new(-2.562, -1.63, 6.269),
        _ => DVec3::new(2.56, -1.63, 6.268),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actuator_limits_hydraulics_and_step_convergence() {
        let run = |hz: u32| {
            let mut f = Flight::runway();
            f.surface = DVec3::new(1., -0.8, 0.3);
            f.spool = 1.;
            f.engine = true;
            f.systems.sr71.canopy_command = true;
            for _ in 0..hz * 8 {
                advance(&mut f, 1. / hz as f64, 3.2);
            }
            f
        };
        let a = run(60);
        let b = run(240);
        for i in 0..4 {
            assert!((a.systems.sr71.elevons[i] - b.systems.sr71.elevons[i]).abs() < 1e-10);
            assert!(a.systems.sr71.elevons[i] <= 0.);
        }
        assert_eq!(a.systems.sr71.canopy, 1.);
        assert!((a.systems.sr71.spike - 0.66).abs() < 1e-10);
        let mut f = Flight::runway();
        f.damage.hydraulics = 0.;
        f.surface = DVec3::ONE;
        advance(&mut f, 1., 3.2);
        assert_eq!(f.systems.sr71.elevons, [0.; 4]);
        assert_eq!(f.systems.sr71.spike, 0.);
    }
    #[test]
    fn gear_moves_continuously_to_internal_stow_without_twisting_nose() {
        let n = gear_rotation(0., 0) * (wheel(0) - mount(0)) + mount(0);
        assert!(n.z < -7.1 && n.y > -0.1);
        assert!(n.x.abs() < 1e-9);
        for i in 1..3 {
            let p = gear_rotation(0., i) * (wheel(i) - mount(i)) + mount(i);
            assert!(p.x.abs() < 1.);
            assert!(p.y > 0.1);
        }
        for i in 0..3 {
            let mut old = wheel(i);
            for step in 1..=100 {
                let e = 1. - step as f64 / 100.;
                let p = mount(i) + gear_rotation(e, i) * (wheel(i) - mount(i));
                assert!(p.distance(old) < 0.06);
                old = p;
            }
        }
    }
}
