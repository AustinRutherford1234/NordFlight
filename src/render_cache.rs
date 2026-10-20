//! Avoid flagging unchanged materials for GPU extraction and upload.
use bevy::prelude::*;

/// Keep ordinary display cadence, but refresh immediately on a scene/view cut.
#[derive(Default)]
pub struct PresentationClock {
    last: f32,
    scene: Option<(u64, usize, bool, Option<usize>, bool)>,
}
impl PresentationClock {
    pub fn due(&mut self, now: f32, period: f32, view: &crate::ViewState) -> bool {
        let scene=(view.generation,view.index,view.map,view.follow_identity(),view.simple);
        if self.scene==Some(scene) && now-self.last<period { return false; }
        self.last=now;
        self.scene=Some(scene);
        true
    }
}
pub fn optics(
    materials: &mut Assets<StandardMaterial>,
    handle: &Handle<StandardMaterial>,
    color: Color,
    emissive: LinearRgba,
) -> bool {
    if !materials
        .get(handle)
        .is_some_and(|m| m.base_color != color || m.emissive != emissive)
    {
        return false;
    }
    if let Some(m) = materials.get_mut(handle) {
        m.base_color = color;
        m.emissive = emissive;
    }
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presentation_cadence_preserves_period_but_never_defers_scene_cuts() {
        let mut gate=PresentationClock::default();
        let mut view=crate::ViewState::default();
        assert!(gate.due(1.,0.1,&view));
        assert!(!gate.due(1.01,0.1,&view));
        view.generation+=1;
        assert!(gate.due(1.02,0.1,&view));
        assert!(!gate.due(1.03,0.1,&view));
        view.index=3;
        assert!(gate.due(1.04,0.1,&view));
        assert!(gate.due(1.15,0.1,&view));
    }
    #[test]
    fn unchanged_optics_do_not_modify_assets() {
        use bevy::asset::{AssetApp, AssetEvent, AssetPlugin};
        let mut app = App::new();
        app.add_plugins((bevy::app::TaskPoolPlugin::default(), AssetPlugin::default()))
            .init_asset::<StandardMaterial>();
        let h = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        app.update();
        app.world_mut()
            .resource_mut::<Messages<AssetEvent<StandardMaterial>>>()
            .drain()
            .for_each(drop);
        for _ in 0..100 {
            assert!(!optics(
                &mut app.world_mut().resource_mut::<Assets<StandardMaterial>>(),
                &h,
                Color::WHITE,
                LinearRgba::BLACK
            ));
        }
        app.update();
        assert!(
            !app.world_mut()
                .resource_mut::<Messages<AssetEvent<StandardMaterial>>>()
                .drain()
                .any(|e| matches!(e, AssetEvent::Modified { .. }))
        );
        assert!(optics(
            &mut app.world_mut().resource_mut::<Assets<StandardMaterial>>(),
            &h,
            Color::BLACK,
            LinearRgba::BLACK
        ));
        app.update();
        assert!(
            app.world_mut()
                .resource_mut::<Messages<AssetEvent<StandardMaterial>>>()
                .drain()
                .any(|e| matches!(e, AssetEvent::Modified { .. }))
        );
    }
}
