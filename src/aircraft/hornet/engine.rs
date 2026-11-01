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
/// Engineering blend: engine suction dominates low-speed inlet direction.
/// Freestream incidence becomes meaningful as dynamic pressure builds. These
/// bounds are estimates, not measured F414 compressor stability data.
fn inlet_distortion(air: DVec3, density: f64) -> f64 {
    let speed = air.length();
    if speed < 1e-6 { return 0.; }
    let q = 0.5 * density.max(0.) * speed * speed;
    let blend = ((q - 250.) / (4000. - 250.)).clamp(0., 1.);
    let blend = blend * blend * (3. - 2. * blend);
    let alpha = (-air.y).atan2(-air.z);
    let beta = (air.x / speed).clamp(-1., 1.).asin();
    blend * ((alpha.abs() - 0.65).max(0.) + (beta.abs() - 0.3).max(0.) * 1.5)
}
pub fn advance(f: &mut Flight, dt: f64) -> Loads {
    let altitude = f.altitude();
    let mut result = Loads::default();
    let (cg, _) = mass::properties(f);
    let mut flow = 0.;
    let mut spool = 0.;
    let fueled = f.fuel + super::stores::fuel(f) > 0. || f.infinite_fuel;
    let air = f.orientation.conjugate() * (f.velocity - f.wind);
    let distortion = inlet_distortion(air, f.density);
    for i in 0..2 {
        let health = f.damage.engines[i].clamp(0., 1.);
        let state = &mut f.hornet.engines[i];
        state.distortion_time = if distortion > 0.6 {
            state.distortion_time + dt
        } else {
            (state.distortion_time - dt * 2.).max(0.)
        };
        if state.distortion_time > 1.2 || health < 0.12 {
            state.flameout = true;
        }
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
        // Spool here is normalized commanded power, not physical N2 RPM.
        // Running idle (0.06) must continue driving the hydraulic pumps.
        .any(|e| e.thrust > 0. && e.spool > 0.025)
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
        super::stores::burn(f, dt * (flow + f.damage.fuel_leak));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(f:&mut Flight,seconds:f64,hz:usize) {
        for _ in 0..(seconds*hz as f64)as usize {
            f.airspeed=(f.velocity-f.wind).length(); advance(f,1./hz as f64);
        }
    }
    #[test]
    fn carrier_airflow_never_shuts_down_a_healthy_taxiing_engine() {
        for hz in [60,120,240] { for heading in [0f64,45.,90.,135.,180.,225.,270.,315.] {
            let mut f=crate::carrier::scenario(true);
            f.orientation=crate::carrier::rotation(0.)*DQuat::from_rotation_y(heading.to_radians());
            f.apu=false;f.throttle=0.3;
            run(&mut f,15.,hz);
            assert!(f.hornet.engines.iter().all(|e|!e.flameout&&e.thrust>1000.&&e.flow>0.),"heading={heading} hz={hz}");
            assert!(f.hornet.hydraulic_pressure>0.9);
        }}
    }
    #[test]
    fn static_and_low_speed_tail_crosswinds_allow_idle_and_full_power() {
        for speed in [0f64,10.,20.,30.] { for angle in [0f64,90.,180.,270.] {for power in [0.,1.] {
            let mut f=crate::carrier::scenario(true);f.apu=false;f.throttle=power;
            f.velocity=f.orientation*(DQuat::from_rotation_y(angle.to_radians())*DVec3::NEG_Z*speed);f.wind=DVec3::ZERO;
            run(&mut f,15.,120);
            assert!(f.hornet.engines.iter().all(|e|!e.flameout&&e.thrust>0.),"speed={speed} angle={angle} power={power}");
        }}}
    }
    #[test]
    fn real_high_speed_distortion_failure_and_restart_still_apply() {
        let mut f=crate::carrier::scenario(true);f.apu=false;f.throttle=0.6;
        f.velocity=f.orientation*DVec3::Z*180.;f.wind=DVec3::ZERO;
        run(&mut f,3.,120);assert!(f.hornet.engines.iter().all(|e|e.flameout&&e.thrust==0.));
        f.velocity=DVec3::ZERO;f.apu=true;
        run(&mut f,6.,120);assert!(f.hornet.engines.iter().all(|e|!e.flameout&&e.thrust>0.));
        f.damage.engines[0]=0.05;run(&mut f,1.,120);assert_eq!(f.hornet.engines[0].thrust,0.);assert!(f.hornet.engines[1].thrust>0.);
        f.engine=false;run(&mut f,1.,120);assert_eq!(f.thrust,0.);
        f.engine=true;f.fuel=0.;f.hornet.tanks=Default::default();run(&mut f,1.,120);assert_eq!(f.thrust,0.);
    }
    #[test]
    fn distortion_is_continuous_and_uses_air_relative_velocity() {
        assert_eq!(inlet_distortion(DVec3::ZERO,1.225),0.);
        assert_eq!(inlet_distortion(DVec3::Z*12.86,1.225),0.);
        for q in [250f64,4000.] {
            let a=inlet_distortion(DVec3::Z*(2.*(q-0.001)/1.225).sqrt(),1.225);
            let b=inlet_distortion(DVec3::Z*(2.*(q+0.001)/1.225).sqrt(),1.225);
            assert!((b-a).abs()<1e-6);
        }
    }
}
