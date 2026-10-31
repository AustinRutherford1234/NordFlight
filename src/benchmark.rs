//! Diagnostic-only input scripts. Ordinary flight never enters these paths.
use crate::{ViewState, combat, physics::*, space, warp};
use bevy::prelude::*;

#[derive(Resource)]
pub struct Session {
    pub script: String,
    pub soak: bool,
    atmosphere_only: bool,
    pub warmup: f64,
    pub seconds: f64,
    pub events: Vec<serde_json::Value>,
    /// Diagnostic-only ablation: retain the last rendered instrument pages.
    pub instruments_frozen: bool,
    phase: Option<usize>,
    phase_seconds: f64,
    last_event: u64,
    last_reset: f64,
    next_sample: f64,
}
fn number(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|x| x.is_finite() && *x > 0.)
        .unwrap_or(default)
}
pub fn terrain_origin() -> (f64,f64) {
    let read=|name,default|std::env::var(name).ok().and_then(|s|s.parse::<f64>().ok())
        .filter(|v|v.is_finite()).unwrap_or(default);
    (read("NORD_TERRAIN_X",-8000.),read("NORD_TERRAIN_Z",-1000.))
}
impl Default for Session {
    fn default() -> Self {
        Self {
            script: std::env::var("NORD_BENCH_SCRIPT").unwrap_or_default(),
            soak: std::env::var_os("NORD_BENCH_SOAK").is_some(),
            atmosphere_only: std::env::var_os("NORD_SOAK_ATMOSPHERE").is_some(),
            warmup: number("NORD_BENCH_WARMUP", 20.),
            seconds: number("NORD_BENCH_SECONDS", 40.),
            events: Vec::new(),
            instruments_frozen: false,
            phase: None,
            phase_seconds: number("NORD_SOAK_SEGMENT_SECONDS", 150.),
            last_event: u64::MAX,
            last_reset: 0.,
            next_sample: 0.,
        }
    }
}
pub fn configured_warp(active: bool) -> warp::Warp {
    if !active {
        return warp::Warp::load();
    }
    let mut w = warp::Warp::default();
    w.timestep = number("NORD_BENCH_TIMESTEP", 1. / 60.).clamp(0.001, 0.05);
    w
}
fn phase_name(phase: usize) -> &'static str {
    [
        "bvr", "combat", "terrain", "gear", "earthmap", "ascent", "descent", "landing", "warp",
        "moonmap", "cockpit", "effects",
    ][phase % 12]
}
fn initialize(
    script: &str,
    f: &mut Flight,
    v: &mut ViewState,
    e: &mut Environment,
    w: &mut warp::Warp,
    c: &mut combat::Combat,
    menu: &mut combat::bvr::Menu,
) {
    let scenario = match script {
        "bvr" => 8,
        "combat" | "effects" => 6,
        "terrain" => 2,
        "gear" => 1,
        "landing" => 10,
        "moonmap" => 9,
        _ => 3,
    };
    crate::reset(f, scenario, v);
    *e = Environment::default();
    if !matches!(scenario, 4..=8) { *c=combat::Combat::default(); }
    w.normal();
    w.effective = 1.;
    c.diagnostic_controls(f, false, false, false);
    match script {
        "bvr" => {
            menu.formation = 3;
            combat::bvr::start(c, f, v.generation, menu);
            v.index = 3;
        }
        "combat" | "effects" => {
            c.start_air(f, v.generation, 6);
            f.throttle = 0.9;
        }
        "terrain" => {
            f.position = crate::terrain::point(10000., 0., terrain_height(10000., 0.) + 350.);
            f.gear = false;
            f.throttle = 0.8;
        }
        "gear" => {
            f.position = surface_point(0., 0., 3000.);
            f.velocity = f.forward() * 230.;
            f.landed = false;
            f.throttle = 0.8;
        }
        "ascent" => {
            f.position = surface_point(0., 0., 12000.);
            f.velocity = f.up() * 2500. + f.forward() * 1000.;
            f.throttle = 0.;
            f.engine = false;
        }
        "atmosphere" => {
            f.position = surface_point(0., 0., 60000.);
            f.velocity = f.forward()*900.;
            f.engine=false;
            f.throttle=0.;
        }
        "descent" => {
            f.space.stage = space::Stage::Capsule;
            f.fuel = 0.;
            f.position = surface_point(0., 0., 120000.);
            f.velocity = f.forward() * 7000. - f.up() * 1000.;
            f.orientation =
                bevy::math::DQuat::from_rotation_arc(bevy::math::DVec3::Z, f.velocity.normalize());
            f.space.chute_armed = true;
        }
        "landing" => {
            let up = f.up();
            f.position -= up * 1200.;
            v.index = 1;
        }
        "warp" => {
            f.throttle = 0.;
            f.spool = 0.;
            f.thrust = 0.;
            f.rates = bevy::math::DVec3::ZERO;
            f.controls = bevy::math::DVec3::ZERO;
            w.selected = w.rates.len() - 1;
            v.map = true;
        }
        "moonmap" | "earthmap" => {
            v.map = true;
        }
        "cockpit" => {
            crate::reset(f, 1, v);
            v.index = 3;
        }
        _ => {}
    }
    f.paused = false;
}
pub fn neutral_inputs(benchmark: Res<crate::RenderBenchmark>,mut keys: ResMut<ButtonInput<KeyCode>>,mut mouse: ResMut<ButtonInput<MouseButton>>) {
    if benchmark.mode.is_some() { keys.reset_all(); mouse.reset_all(); }
}
pub fn exercise(
    time: Res<Time<bevy::time::Real>>,
    mut session: ResMut<Session>,
    benchmark: Res<crate::RenderBenchmark>,
    mut f: ResMut<Flight>,
    mut v: ResMut<ViewState>,
    mut e: ResMut<Environment>,
    mut w: ResMut<warp::Warp>,
    mut c: ResMut<combat::Combat>,
    mut menu: ResMut<combat::bvr::Menu>,
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    entities: Query<Entity>,
) {
    if benchmark.mode.is_none() {
        return;
    }
    let wall = time.elapsed_secs_f64();
    session.instruments_frozen = std::env::var_os("NORD_BENCH_FREEZE_INSTRUMENTS").is_some()
        && wall >= session.warmup;
    let phase = (wall / session.phase_seconds) as usize;
    let script = if session.soak {
        if session.atmosphere_only {
            ["bvr","cockpit","terrain","gear","atmosphere","effects"][phase%6].to_string()
        } else { phase_name(phase).to_string() }
    } else {
        session.script.clone()
    };
    if std::env::var_os("NORD_NAVAL_CAPTURE").is_some(){
        if wall>=session.next_sample {session.next_sample=wall+1.;session.events.push(serde_json::json!({"wall_s":wall,"simulation_s":f.elapsed,"entities":entities.iter().count(),"meshes":meshes.len(),"images":images.len(),"naval":c.naval.as_ref().map(|n|serde_json::json!({"waves":n.waves,"tracks":n.tracks.len(),"round_slots":n.rounds.len(),"active_rounds":n.rounds.iter().filter(|m|m.alive).count(),"carrier_hull":n.ships[0].hull}))}));}return;
    }
    let scripted =
        session.soak || matches!(script.as_str(), "ascent" | "descent" | "landing" | "warp");
    if script == "carrier-live" {
        // Initialize once, then leave position, attitude and velocity entirely
        // to the ordinary simulation. Never recycle a crashed flight.
        if session.phase.is_none() {
            *f = crate::carrier::scenario(false);
            f.paused = false;
            v.scenario = 11;
            v.index = if std::env::var("NORD_BENCH_CAMERA").as_deref() == Ok("external") { 1 } else { 3 };
            v.yaw = 0.;
            v.pitch = -0.1;
            e.wind_enabled = false;
            session.phase = Some(0);
        }
        if wall >= session.next_sample {
            session.next_sample = wall + 1.;
            let relative = crate::carrier::relative(f.position, f.elapsed);
            let frozen = session.instruments_frozen;
            session.events.push(serde_json::json!({
                "wall_s": wall, "simulation_s": f.elapsed,
                "altitude_m": f.altitude(), "agl_m": f.agl(),
                "carrier_relative_m": relative.to_array(),
                "speed_mps": f.velocity.length(), "alpha_rad": f.alpha,
                "throttle": f.throttle, "crashed": f.crashed,
                "landed": f.landed, "paused": f.paused,
                "camera": v.index, "meshes": meshes.len(),
                "entities": entities.iter().count(),
                "instruments_frozen": frozen
            }));
        }
        return;
    }
    if script=="ocean" {
        // Approved render comparison: move the formation and wave clock without
        // integrating flight physics, identically in the reference and new path.
        let previous_time=f.elapsed;
        f.position+=crate::carrier::origin(wall)-crate::carrier::origin(previous_time);
        f.elapsed=wall;f.paused=true;f.orientation=crate::carrier::rotation(wall);
        return;
    }
    if script=="carrier-approach" {
        // Diagnostic-only repeated final approach; never alters normal flight.
        let z=2300.-(wall*70.)%2000.;
        if session.phase.is_none(){*f=crate::carrier::scenario(false);v.scenario=11;session.phase=Some(0);}
        f.position=crate::carrier::world(bevy::math::DVec3::new(crate::carrier::lane(z),crate::carrier::DECK+(z-75.)*0.061+3.,z),wall);
        f.orientation=crate::carrier::rotation(wall)*bevy::math::DQuat::from_rotation_y(0.14)*bevy::math::DQuat::from_rotation_x(0.07);
        f.elapsed=wall;f.paused=true;v.index=if std::env::var_os("NORD_TERRAIN_COCKPIT").is_some(){3}else{1};
        if wall>=session.next_sample{session.next_sample=wall+1.;session.events.push(serde_json::json!({"wall_s":wall,"carrier_range_m":z,"camera":v.index,"meshes":meshes.len(),"entities":entities.iter().count()}));}
        return;
    }
    if script == "terrain-stream" {
        // A deterministic camera/streaming path isolates presentation from
        // aircraft collisions. Ordinary flight never enters this branch.
        let agl = number("NORD_TERRAIN_AGL", 5.);
        let (x,z) = terrain_origin();
        let z = z + wall * number("NORD_TERRAIN_SPEED", 100.);
        f.position = crate::terrain::point(x, z, terrain_height(x, z).max(0.) + agl);
        f.paused = true;
        if wall >= session.next_sample {
            session.next_sample = wall + 1.;
            session.events.push(serde_json::json!({"wall_s":wall,"agl_m":f.agl(),
                "meshes":meshes.len(),"entities":entities.iter().count(),"x":x,"z":z,
                "forest":crate::terrain::forest(x,z),"ground_m":terrain_height(x,z)}));
        }
        return;
    }
    if !scripted {
        return;
    }
    if session.phase != Some(phase)
        || (f.crashed && wall - session.last_reset > 5.)
        || (session.soak
            && matches!(script.as_str(), "combat" | "effects")
            && wall - session.last_reset > 25.)
    {
        initialize(&script, &mut f, &mut v, &mut e, &mut w, &mut c, &mut menu);
        session.phase = Some(phase);
        session.last_reset = wall;
        session.last_event = u64::MAX;
        session
            .events
            .push(serde_json::json!({"event":"reset","phase":phase,"script":script,"wall_s":wall}));
    }
    let local = wall % session.phase_seconds;
    if script == "landing" && !f.landed {
        let desired = -((f.agl() - 8.5).max(0.) * 0.8).sqrt().clamp(0.2, 20.);
        let g = space::MOON_MU / (f.altitude() + space::MOON_RADIUS).powi(2);
        f.throttle = if f.space.legs.iter().filter(|x| **x > 0.).count() >= 2 {
            0.
        } else {
            (f.mass() * (g + 1.8 * (desired - f.vertical_speed())) / f.space.stage.engine().0)
                .clamp(0., 1.)
        };
    }
    if session.soak {
        c.diagnostic_controls(
            &f,
            matches!(script.as_str(), "combat" | "effects") && local_pulse(wall),
            false,
            false,
        );
        let event = (local / 4.) as u64;
        // Exercise pause, both map paths and four flight cameras without changing
        // the production input or simulation integrator.
        f.paused = local > 35. && local < 37.;
        if !matches!(script.as_str(), "warp" | "earthmap" | "moonmap") {
            v.map = local > 60. && local < 66.;
            v.index = (local / 20.) as usize % 4;
        }
        if script == "gear" {
            f.gear = (local / 7.) as usize % 2 == 0;
        }
        w.timestep = if local > 80. && local < 100. {
            0.01
        } else {
            1. / 60.
        };
        if event != session.last_event {
            c.diagnostic_controls(
                &f,
                matches!(script.as_str(), "combat" | "effects"),
                true,
                true,
            );
            if script == "effects" && event % 3 == 1 {
                c.diagnostic_damage();
            }
            session.last_event = event;
        }
    }
    if wall >= session.next_sample {
        assert!(
            f.position.is_finite() && f.velocity.is_finite() && f.orientation.is_finite(),
            "nonfinite diagnostic flight state"
        );
        assert!(
            c.fx.particles.len() <= 256,
            "effect pool exceeded its bound"
        );
        session.next_sample = wall + 5.;
        session.events.push(serde_json::json!({"event":"sample","wall_s":wall,"script":script,"phase":phase,
            "simulation_s":f.elapsed,"paused":f.paused,"crashed":f.crashed,"landed":f.landed,
            "altitude_m":f.altitude(),"warp":w.effective,"timestep":w.timestep,
            "entities":entities.iter().count(),"meshes":meshes.len(),"images":images.len(),
            "aircraft":c.targets.len(),"missiles":c.missiles.len(),"particles":c.fx.particles.len()}));
    }
}
fn local_pulse(wall: f64) -> bool {
    wall % 6. < 2.
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scripts_cover_all_workloads_and_use_normal_resets() {
        let mut f = Flight::default();
        let mut v = ViewState::default();
        let mut e = Environment::default();
        let mut w = warp::Warp::default();
        let mut c = combat::Combat::default();
        let mut menu = combat::bvr::Menu::default();
        for phase in 0..12 {
            initialize(
                phase_name(phase),
                &mut f,
                &mut v,
                &mut e,
                &mut w,
                &mut c,
                &mut menu,
            );
            assert!(f.position.is_finite() && f.velocity.is_finite() && !f.paused);
            if phase_name(phase) == "bvr" {
                assert_eq!(c.targets.len(), 11);
            }
            if phase_name(phase) == "warp" {
                assert_eq!(w.allowed(&f), 100000.);
            }
        }
    }
}
