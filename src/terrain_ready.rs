//! A complete terrain replacement is published only after its GPU meshes exist.
use bevy::{
    asset::AssetId,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        mesh::RenderMesh,
        render_asset::{RenderAssets, prepare_assets},
    },
};
use std::sync::{Arc, Mutex};
#[derive(Default)]
struct State {
    ids: Vec<AssetId<Mesh>>,
    ready: bool,
}
#[derive(Resource, Clone, Default)]
pub struct TerrainReady(Arc<Mutex<State>>);
impl TerrainReady {
    pub fn request(&self, ids: Vec<AssetId<Mesh>>) {
        *self.0.lock().unwrap() = State { ids, ready: false };
    }
    pub fn ready(&self) -> bool {
        self.0.lock().unwrap().ready
    }
    pub fn clear(&self) {
        *self.0.lock().unwrap() = State::default();
    }
    pub(crate) fn poll(&self, prepared: impl Fn(AssetId<Mesh>)->bool) {
        let mut state=self.0.lock().unwrap();
        if !state.ready && !state.ids.is_empty() {state.ready=state.ids.iter().copied().all(prepared);}
    }
}
pub struct TerrainReadyPlugin;
impl Plugin for TerrainReadyPlugin {
    fn build(&self, app: &mut App) {
        let ready = TerrainReady::default();
        app.insert_resource(ready.clone());
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.insert_resource(ready).add_systems(
                Render,
                check
                    .in_set(RenderSystems::PrepareAssets)
                    .after(prepare_assets::<RenderMesh>),
            );
        }
    }
}
fn check(state: Res<TerrainReady>, meshes: Res<RenderAssets<RenderMesh>>) {
    state.poll(|id|meshes.get(id).is_some());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_requests_and_retirement_discard_previous_readiness() {
        let state = TerrainReady::default();
        state.0.lock().unwrap().ready = true;
        state.request(vec![Handle::<Mesh>::default().id()]);
        assert!(!state.ready());
        assert_eq!(state.0.lock().unwrap().ids.len(), 1);
        state.0.lock().unwrap().ready = true;
        state.clear();
        assert!(!state.ready());
        assert!(state.0.lock().unwrap().ids.is_empty());
    }
}
