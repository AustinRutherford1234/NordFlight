use super::*;
#[derive(Clone, Copy, Debug)]
pub struct Engine {
    pub spool: f64,
    pub temperature: f64,
    pub flameout: bool,
    pub thrust: f64,
    pub flow: f64,
    pub distortion_time: f64,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            spool: 0.,
            temperature: 288.,
            flameout: false,
            thrust: 0.,
            flow: 0.,
            distortion_time: 0.,
        }
    }
}
pub fn position(i: usize) -> DVec3 {
    DVec3::new(if i == 0 { -0.475 } else { 0.475 }, 0.48, 8.202)
}
pub fn output(power: f64, rho: f64, mach: f64) -> (f64, f64) {
    let lapse = config::table(
        rho,
        &[0., 0.04, 0.19, 0.41, 1.225],
        &[0., 0.10, 0.32, 0.56, 1.],
    );
    let ram = config::table(
        mach,
        &[0., 0.6, 0.9, 1.2, 1.8, 2.2],
        &[1., 0.94, 0.98, 1.10, 1.18, 0.95],
    );
    let burner = ((power - 0.88) / 0.12).clamp(0., 1.);
    let dry = config::MIL * (power / 0.88).clamp(0., 1.);
    let wet = (config::MAX - config::MIL) * burner;
    (
        (dry + wet) * lapse * ram,
        (dry * 0.000021 + wet * 0.000065) * lapse * ram,
    )
}
pub fn advance(f: &mut Flight, dt: f64) -> Loads {
    let altitude = f.altitude();
    let mut result = Loads::default();
    let (cg, _) = mass::properties(f);
    let mut flow = 0.;
    let mut spool = 0.;
    for i in 0..2 {
        let health = f.damage.engines[i].clamp(0., 1.);
        let state = &mut f.hornet.engines[i];
        let distortion = (f.alpha.abs() - 0.65).max(0.) + (f.beta.abs() - 0.3).max(0.) * 1.5;
        state.distortion_time = if distortion > 0.6 {
            state.distortion_time + dt
        } else {
            (state.distortion_time - dt * 2.).max(0.)
        };
        if state.distortion_time > 1.2 || health < 0.12 {
            state.flameout = true;
        }
        let fueled = f.fuel > 0. || f.infinite_fuel;
        let running = f.engine && f.systems.engine_on[i] && fueled && health > 0.12;
        if state.flameout
            && running
            && distortion < 0.2
            && altitude < 12000.
            && (f.apu || f.airspeed > 80.)
        {
            state.flameout = false;
        }
        let target = if running && !state.flameout {
            f.throttle.max(0.06)
        } else {
            0.
        };
        let tau = if target > state.spool { 1.4 } else { 0.9 };
        state.spool += (target - state.spool) * (1. - (-dt / tau).exp());
        let (thrust, fuel) = output(state.spool, f.density, f.mach);
        let recovery = (1. - 0.22 * distortion).clamp(0.45, 1.);
        state.thrust = if running && !state.flameout {
            thrust * health * recovery
        } else {
            0.
        };
        state.flow = if state.thrust > 0. { fuel * health } else { 0. };
        let temp = if state.thrust > 0. {
            450. + 650. * state.spool + 200. * (1. - health)
        } else {
            288.
        };
        state.temperature += (temp - state.temperature) * (1. - (-dt / 12.).exp());
        if state.temperature > 1120. {
            f.damage.engines[i] = (health - dt * (state.temperature - 1120.) * 0.00001).max(0.);
        }
        let air = f.orientation.conjugate() * (f.velocity - f.wind);
        let windmill = if state.thrust == 0. {
            -air.normalize_or_zero() * (f.density * f.airspeed.powi(2) * 0.018)
        } else {
            DVec3::ZERO
        };
        let force = DVec3::NEG_Z * state.thrust + windmill;
        result.add(force, position(i) - cg);
        flow += state.flow;
        spool += state.spool;
        f.systems.engines[i] = health;
    }
    let hydraulic = if f
        .hornet
        .engines
        .iter()
        .any(|e| e.thrust > 0. && e.spool > 0.15)
    {
        1.
    } else if f.apu {
        0.4
    } else {
        0.
    };
    f.hornet.hydraulic_pressure +=
        (hydraulic - f.hornet.hydraulic_pressure) * (1. - (-dt / 3.).exp());
    f.spool = spool * 0.5;
    f.thrust = f.hornet.engines.iter().map(|e| e.thrust).sum();
    if !f.infinite_fuel {
        f.fuel = (f.fuel - dt * (flow + f.damage.fuel_leak)).max(0.);
    }
    result
}
