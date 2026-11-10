//! A complete terrain replacement is published only after its GPU meshes exist.
use bevy::{
    asset::{AssetId, UntypedAssetId},
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
    images: Vec<AssetId<Image>>,
    materials: Vec<UntypedAssetId>,
    requested: bool,
    ready: bool,
}
#[derive(Resource, Clone, Default)]
pub struct TerrainReady(Arc<Mutex<State>>);
impl TerrainReady {
    pub fn request(&self, ids: Vec<AssetId<Mesh>>) {
        *self.0.lock().unwrap() = State {
            ids,
            requested: true,
            ready: false,
            ..Default::default()
        };
    }
    pub fn dependencies(&self, images: Vec<AssetId<Image>>, materials: Vec<UntypedAssetId>) {
        let mut s = self.0.lock().unwrap();
        s.images = images;
        s.materials = materials;
        s.ready = false;
    }
    /// Replace a complete loading snapshot when its assets change. Set order
    /// and duplicate scene references do not invalidate renderer readiness.
    pub(crate) fn request_if_changed(
        &self,
        mut ids: Vec<AssetId<Mesh>>,
        mut images: Vec<AssetId<Image>>,
        mut materials: Vec<UntypedAssetId>,
    ) -> bool {
        ids.sort_unstable();
        ids.dedup();
        images.sort_unstable();
        images.dedup();
        materials.sort_unstable();
        materials.dedup();
        let mut state = self.0.lock().unwrap();
        if state.requested && state.ids == ids && state.images == images && state.materials == materials {
            return false;
        }
        *state = State { ids, images, materials, requested: true, ready: false };
        true
    }
    pub fn ready(&self) -> bool {
        self.0.lock().unwrap().ready
    }
    pub fn clear(&self) {
        *self.0.lock().unwrap() = State::default();
    }
    #[cfg(test)]
    pub(crate) fn poll(&self, prepared: impl Fn(AssetId<Mesh>) -> bool) {
        self.poll_all(prepared, |_| true, |_| true);
    }
    pub(crate) fn poll_all(
        &self,
        meshes: impl Fn(AssetId<Mesh>) -> bool,
        images: impl Fn(AssetId<Image>) -> bool,
        materials: impl Fn(UntypedAssetId) -> bool,
    ) {
        let mut s = self.0.lock().unwrap();
        if s.requested && !s.ready {
            s.ready = s.ids.iter().copied().all(meshes)
                && s.images.iter().copied().all(images)
                && s.materials.iter().copied().all(materials);
        }
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
fn check(
    state: Res<TerrainReady>,
    meshes: Res<RenderAssets<RenderMesh>>,
    images: Res<RenderAssets<bevy::render::texture::GpuImage>>,
    materials: Res<
        bevy::render::erased_render_asset::ErasedRenderAssets<bevy::pbr::PreparedMaterial>,
    >,
) {
    state.poll_all(
        |id| meshes.get(id).is_some(),
        |id| images.get(id).is_some(),
        |id| materials.get(id).is_some(),
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn meshes_cannot_publish_before_texture_and_material_dependencies() {
        let state = TerrainReady::default();
        state.request(vec![Handle::<Mesh>::default().id()]);
        state.dependencies(
            vec![Handle::<Image>::default().id()],
            vec![Handle::<StandardMaterial>::default().id().untyped()],
        );
        state.poll_all(|_| true, |_| false, |_| true);
        assert!(!state.ready());
        state.poll_all(|_| true, |_| true, |_| false);
        assert!(!state.ready());
        state.poll_all(|_| true, |_| true, |_| true);
        assert!(state.ready());
    }
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
    #[test]
    fn an_explicit_empty_request_can_complete_but_a_cleared_request_cannot() {
        let state = TerrainReady::default();
        assert!(state.request_if_changed(vec![], vec![], vec![]));
        state.poll(|_| false);
        assert!(state.ready());
        assert!(!state.request_if_changed(vec![], vec![], vec![]));
        state.clear();
        state.poll(|_| true);
        assert!(!state.ready());
    }
}
