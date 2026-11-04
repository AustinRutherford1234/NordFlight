//! Geographic scenario anchors and pack requirements; no terrain carving.
use bevy::math::DVec3;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDefinition {
    pub version: u32,
    pub id: String,
    pub pack: String,
    pub longitude: f64,
    pub latitude: f64,
    pub altitude_agl: f64,
    pub heading_degrees: f64,
}
pub fn definitions() -> &'static [ScenarioDefinition] {
    static DEFS: std::sync::OnceLock<Vec<ScenarioDefinition>> = std::sync::OnceLock::new();
    DEFS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/environment/scenarios.json")).unwrap()
    })
}
pub fn position(id: &str) -> Option<DVec3> {
    if !crate::environment::real() {
        return None;
    }
    let d = definitions().iter().find(|d| d.id == id)?;
    let mut p =
        crate::environment::geo_to_local(d.longitude, d.latitude, 0., crate::environment::radius());
    let sample = crate::environment::active().sample_surface(p);
    p = p.normalize()
        * (crate::environment::radius()
            + sample.water_surface.unwrap_or(sample.ground_height)
            + d.altitude_agl);
    Some(p)
}
pub fn validate() -> Result<(), String> {
    if !crate::environment::real() {
        return Ok(());
    }
    for d in definitions() {
        if d.version != 1
            || !d.longitude.is_finite()
            || !d.latitude.is_finite()
            || d.latitude.abs() > 90.
            || !d.altitude_agl.is_finite()
        {
            return Err("invalid scenario".into());
        }
        let p = crate::environment::geo_to_local(
            d.longitude,
            d.latitude,
            0.,
            crate::environment::radius(),
        );
        let sample = crate::environment::active().sample_surface(p);
        if !sample.detailed {
            eprintln!("Scenario {} requires terrain pack {}", d.id, d.pack);
        }
        if d.id == "carrier" && sample.water_surface.is_none() {
            return Err("carrier scenario must begin on real water".into());
        }
    }
    Ok(())
}

pub fn direction(id: &str, position: DVec3) -> Option<DVec3> {
    if !crate::environment::real() {
        return None;
    }
    let scenario = definitions().iter().find(|d| d.id == id)?;
    let (north, east) = crate::physics::geographic_axes(position.normalize());
    let heading = scenario.heading_degrees.to_radians();
    Some(north * heading.cos() + east * heading.sin())
}

/// Unopposed mountain flying. Keep the first ~40 seconds clear of ridges,
/// while leaving the pilot free to descend and follow valleys afterward.
pub fn mountain_flight(aircraft:crate::aircraft::Aircraft) -> crate::physics::Flight {
    use crate::physics::*;
    let mut f=Flight::approach();
    crate::aircraft::select(&mut f,aircraft);
    f.position=position("mountain-freeflight").unwrap_or_else(||surface_point(100000.,0.,5000.));
    let up=f.position.normalize();
    let forward=direction("mountain-freeflight",f.position).unwrap_or(-DVec3::Z);
    let mut altitude=f.altitude();
    for i in 0..=32 {
        let p=(f.position+forward*(i as f64*250.)).normalize()*crate::environment::radius();
        altitude=altitude.max(crate::environment::active().raw(p).0+300.);
    }
    f.position=up*(crate::environment::radius()+altitude);
    f.orientation=bevy::math::DQuat::from_mat3(&bevy::math::DMat3::from_cols(forward.cross(up),up,-forward));
    f.velocity=forward*190.;
    f.gear=false;f.gear_extension=0.;f.gear_door=0.;f.flaps=0.;
    f.throttle=0.65;f.spool=0.65;f.landed=false;f.brakes=false;
    f.airspeed=f.velocity.length();
    let(t,p,r,s)=atmosphere(f.altitude());
    f.temperature=t;f.pressure=p;f.density=r;f.mach=f.airspeed/s;
    f.thrust=crate::aircraft::jet_output(f.aircraft,f.spool,r,f.mach).0;
    f.status="NORWEGIAN MOUNTAINS / FREE FLIGHT · NO HOSTILES".into();
    f
}
