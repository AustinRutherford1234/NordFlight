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
