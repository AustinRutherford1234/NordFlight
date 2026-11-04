//! Native headless checks use the actual selected immutable world and physics.
use crate::physics::*;

pub fn run() -> Result<(), String> {
    let mut atmosphere = Environment::default();
    atmosphere.wind_enabled = false;
    let mut records = vec![];
    for (cold, name) in [(true, "cold_apron"), (false, "runway")] {
        let mut f = if cold {
            Flight::cold()
        } else {
            Flight::runway()
        };
        f.brakes = true;
        f.engine = false;
        f.throttle = 0.;
        f.spool = 0.;
        for _ in 0..1200 {
            simulate(&mut f, &atmosphere, 1. / 240.);
        }
        if f.crashed || !f.position.is_finite() || f.gear_health.iter().any(|h| *h <= 0.) {
            return Err(format!("{name} failed: {} / AGL {}", f.status, f.agl()));
        }
        records.push(serde_json::json!({"case":name,"crashed":f.crashed,"agl_m":f.agl(),"gear_health":f.gear_health,"ground_m":contact_height_at(f.position)}));
    }
    let mut f = Flight::runway();
    f.throttle = 1.;
    f.engine = true;
    f.assist = true;
    f.brakes = false;
    let mut airborne = false;
    for tick in 0..240 * 30 {
        if tick > 240 * 9 {
            f.controls.x = 0.35;
        }
        simulate(&mut f, &atmosphere, 1. / 240.);
        if f.crashed {
            break;
        }
        if !f.landed && f.agl() > 15. {
            airborne = true;
            break;
        }
    }
    if !airborne || f.crashed {
        return Err(format!(
            "takeoff failed: {} / AGL {} / speed {}",
            f.status,
            f.agl(),
            f.airspeed
        ));
    }
    records.push(serde_json::json!({"case":"takeoff","airborne":airborne,"crashed":f.crashed,"agl_m":f.agl(),"airspeed_m_s":f.airspeed,"simulation_s":f.elapsed}));
    let mut mountain=crate::scenarios::mountain_flight(crate::aircraft::Aircraft::Gripen);
    let start_agl=mountain.agl();let mut minimum_agl=start_agl;
    for _ in 0..240*30 {
        simulate(&mut mountain,&atmosphere,1./240.);
        minimum_agl=minimum_agl.min(mountain.agl());
        if mountain.crashed||!mountain.position.is_finite(){return Err("mountain free flight failed".into());}
    }
    records.push(serde_json::json!({"case":"mountain_freeflight","start_agl_m":start_agl,"minimum_agl_m":minimum_agl,"simulation_s":mountain.elapsed,"crashed":mountain.crashed}));
    let report = serde_json::json!({"environment":super::active().preset.id,"revision":super::active().revision,"terrain_bytes":super::active().data.bytes,"cases":records});
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    if let Ok(path) = std::env::var("NORD_ENVIRONMENT_REPORT") {
        std::fs::write(path, &json).map_err(|e| e.to_string())?;
    }
    println!("{json}");
    Ok(())
}
