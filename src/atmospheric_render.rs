//! Presentation-only sky input versions and discontinuity tracking.
use bevy::{prelude::*, pbr::generate::EnvironmentMapScene};
use crate::{FlightCamera, ViewState, physics::{Environment, Flight, Vehicle}, space};

#[derive(Default)]
pub struct SceneTracker {
    cut: Option<(u64, usize, bool, bool, bool, bool, Option<usize>)>,
    inputs: Vec<u32>,
    position: Option<Vec3>,
    generation: u64,
    source: u64,
}
impl SceneTracker {
    fn update(&mut self, cut: (u64, usize, bool, bool, bool, bool, Option<usize>), position: Vec3,
        inputs: &[u32], enabled: bool) -> EnvironmentMapScene {
        let abrupt_sky = self.inputs.len() == inputs.len() && inputs.len() >= 23
            && (self.inputs[20] != inputs[20]
                || (f32::from_bits(self.inputs[19]) - f32::from_bits(inputs[19])).abs() > 0.05
                || (16..19).any(|i| (f32::from_bits(self.inputs[i]) - f32::from_bits(inputs[i])).abs() > 0.02));
        if self.cut != Some(cut) || abrupt_sky || self.position.is_some_and(|p| p.distance(position) > 2500.) {
            self.generation = self.generation.wrapping_add(1);
            self.cut = Some(cut);
        }
        if self.inputs != inputs {
            self.source = self.source.wrapping_add(1);
            self.inputs.clear();
            self.inputs.extend_from_slice(inputs);
        }
        self.position = Some(position);
        EnvironmentMapScene { generation: self.generation, source: self.source, enabled }
    }
}

pub fn synchronize(
    f: Res<Flight>, e: Res<Environment>, v: Res<ViewState>,
    mut tracker: Local<SceneTracker>,
    mut cameras: Query<(&Transform, &mut EnvironmentMapScene), With<FlightCamera>>,
) {
    let enabled = !v.map && !(f.vehicle == Vehicle::Spacecraft
        && (f.space.body == space::Body::Moon || f.altitude() > 100000.));
    for (transform, mut scene) in &mut cameras {
        // The dedicated indirect probe uses a canonical basis. The reference
        // path still consumes camera orientation. Both use exact input bits.
        let mut inputs = [0u32; 23];
        let probe=std::env::var_os("NORD_RENDER_REFERENCE").is_none() && std::env::var_os("NORD_PROBE_REFERENCE").is_none();
        let source_transform=if probe {Transform::from_translation(transform.translation)} else {*transform};
        inputs[..16].copy_from_slice(&source_transform.to_matrix().to_cols_array().map(f32::to_bits));
        inputs[16..19].copy_from_slice(&(crate::frame(&f).conjugate() * e.sun_direction())
            .as_vec3().to_array().map(f32::to_bits));
        inputs[19] = crate::earth::sky_density(f.altitude()).to_bits();
        inputs[20] = e.weather as u32;
        inputs[21] = v.high_quality as u32;
        inputs[22] = enabled as u32;
        let next = tracker.update((v.generation, v.render_index(), v.map, v.high_quality,
            v.benchmark_msaa, enabled, v.follow_identity()), transform.translation, &inputs, enabled);
        if scene.generation != next.generation || scene.source != next.source || scene.enabled != next.enabled {
            *scene = next;
        }
    }
}

/// Bounded, native frame captures around 100 diagnostic transitions. This path
/// is never enabled in ordinary flight and is excluded from FPS comparisons.
#[derive(Resource, Default)]
pub struct TransitionRun {
    folder: Option<std::path::PathBuf>,
    frame: usize,
}
impl TransitionRun {
    pub fn configured() -> Self {
        let folder=std::env::var_os("NORD_TRANSITION_CAPTURE").map(std::path::PathBuf::from);
        if let Some(folder)=&folder { std::fs::create_dir_all(folder).expect("transition capture folder"); }
        Self { folder, frame:0 }
    }
}
pub fn exercise(
    time: Res<Time<bevy::time::Real>>, mut run: ResMut<TransitionRun>,
    mut f: ResMut<Flight>, mut v: ResMut<ViewState>, mut e: ResMut<Environment>,
    mut combat: ResMut<crate::combat::Combat>,
    mut shadows: ResMut<bevy::light::DirectionalLightShadowMap>,
    mut exit: MessageWriter<AppExit>,
) {
    if run.folder.is_none() || time.elapsed_secs_f64()<8. { return; }
    if run.frame >= 1220 {
        exit.write(AppExit::Success);
        return;
    }
    let event=run.frame/12;
    if run.frame%12 == 1 && event<100 {
        match event%12 {
            0 => crate::reset(&mut f,1,&mut v),
            1 => v.index=3,
            2 => v.index=0,
            3 => { crate::reset(&mut f,2,&mut v); v.index=3; },
            4 => v.map=true,
            5 => v.map=false,
            6 => {
                crate::reset(&mut f,6,&mut v);
                combat.start_air(&f,v.generation,6);
                v.aircraft_follow=Some(0);
            },
            7 => { v.aircraft_follow=None; v.index=3; },
            8 => e.weather=if e.weather==2 {0} else {2},
            9 => e.day_phase=if e.day_phase<1. {2.8} else {-0.1},
            10 => { v.index=0; v.yaw+=1.2; v.pitch=-0.8; },
            _ => { v.high_quality=!v.high_quality; shadows.size=if v.high_quality {2048} else {1024}; },
        }
    }
    f.paused=true;
    run.frame+=1;
}
pub fn capture(
    mut commands: Commands, run: Res<TransitionRun>,
    f: Res<Flight>, v: Res<ViewState>,
    scenes: Query<&EnvironmentMapScene,With<FlightCamera>>,
) {
    let Some(folder)=&run.folder else {return};
    if run.frame==0 || run.frame>1200 { return; }
    let tick=run.frame-1;
    // Capture every frame through a complete six-face lighting cycle and the
    // asynchronous terrain commit, not only the immediate camera-cut frame.
    let stem=folder.join(format!("transition-{:03}-{:02}",tick/12,tick%12));
    let scene=scenes.iter().next().copied().unwrap_or_default();
    let state=serde_json::json!({"frame":tick,"transition":tick/12,"offset":tick%12,
        "generation":scene.generation,"source":scene.source,"enabled":scene.enabled,
        "camera":v.index,"map":v.map,"quality":v.high_quality,"scenario":v.scenario,
        "position":f.position.to_array(),"altitude_m":f.altitude()});
    commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(move |event: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
            event.image.clone().try_into_dynamic().expect("transition image").to_rgb8()
                .save(stem.with_extension("png")).expect("transition PNG");
            std::fs::write(stem.with_extension("json"),serde_json::to_vec_pretty(&state).unwrap())
                .expect("transition state");
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_inputs_reuse_and_cuts_invalidate_pending_work() {
        let mut t = SceneTracker::default();
        let cut = (0, 0, false, false, false, true, None);
        let a=t.update(cut, Vec3::ZERO, &[1,2], true);
        let b=t.update(cut, Vec3::ZERO, &[1,2], true);
        assert_eq!((a.generation,a.source),(b.generation,b.source));
        let c=t.update(cut, Vec3::ZERO, &[1,3], true);
        assert_eq!(b.generation,c.generation);
        assert_ne!(b.source,c.source);
        let d=t.update((1,1,false,false,false,true,None), Vec3::ZERO, &[1,3], true);
        assert_ne!(c.generation,d.generation);
        let e=t.update((1,1,false,false,false,true,None), Vec3::X*3000., &[1,3], true);
        assert_ne!(d.generation,e.generation);
    }
}
