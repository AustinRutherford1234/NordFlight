//! Update fixed-size cockpit textures in place instead of recreating GPU images
//! and every material bind group which references them on each instrument tick.
use bevy::{
    asset::AssetId,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        render_asset::{RenderAssets, prepare_assets},
        render_resource::{
            Extent3d, Origin3d, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureFormat,
        },
        renderer::RenderQueue,
        texture::GpuImage,
    },
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

struct Upload {
    size: Extent3d,
    bytes: Vec<u8>,
    staging: Vec<u8>,
    dirty: Option<Rect>,
}
impl Upload {
    fn publish(&mut self, size: Extent3d, bytes: &[u8]) {
        assert_eq!(size.depth_or_array_layers, 1, "cockpit displays are 2D");
        assert_eq!(bytes.len(), size.width as usize * size.height as usize * 4);
        if self.size != size {
            // Old pending bounds cannot be unioned into a smaller replacement.
            self.dirty = Some(Rect::full(size));
            self.size = size;
        } else if let Some(rect) = changed_rect(&self.bytes, bytes, size) {
            self.dirty = Some(self.dirty.map_or(rect, |old| old.union(rect)));
        } else {
            return;
        }
        self.bytes.clear();
        self.bytes.extend_from_slice(bytes);
    }
    fn prepare(&mut self, gpu_size: Extent3d) -> Option<(Rect, Vec<u8>)> {
        // Extraction may still expose the old image during a resize. Keep the
        // complete pending update until the replacement GPU image is prepared.
        if gpu_size != self.size {
            return None;
        }
        let rect = self.dirty.take()?;
        let mut bytes = std::mem::take(&mut self.staging);
        bytes.clear();
        let stride = self.size.width as usize * 4;
        for y in rect.y..rect.bottom {
            let start = y as usize * stride + rect.x as usize * 4;
            bytes.extend_from_slice(&self.bytes[start..start + rect.width() as usize * 4]);
        }
        Some((rect, bytes))
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x: u32,
    y: u32,
    right: u32,
    bottom: u32,
}
impl Rect {
    fn full(size: Extent3d) -> Self {
        Self {
            x: 0,
            y: 0,
            right: size.width,
            bottom: size.height,
        }
    }
    fn union(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }
    fn width(self) -> u32 {
        self.right - self.x
    }
    fn height(self) -> u32 {
        self.bottom - self.y
    }
}
fn changed_rect(old: &[u8], new: &[u8], size: Extent3d) -> Option<Rect> {
    if old.len() != new.len() {
        return Some(Rect::full(size));
    }
    let stride = size.width as usize * 4;
    let mut changed: Option<Rect> = None;
    for (y, (a, b)) in old
        .chunks_exact(stride)
        .zip(new.chunks_exact(stride))
        .enumerate()
    {
        if a == b {
            continue;
        }
        let first = a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .position(|(a, b)| a != b)
            .unwrap() as u32;
        let last = a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .rposition(|(a, b)| a != b)
            .unwrap() as u32
            + 1;
        let row = Rect {
            x: first,
            y: y as u32,
            right: last,
            bottom: y as u32 + 1,
        };
        changed = Some(changed.map_or(row, |r| r.union(row)));
    }
    changed
}
struct Work {
    id: AssetId<Image>,
    rect: Rect,
    bytes: Vec<u8>,
}
#[derive(Resource, Clone, Default)]
pub struct DisplayUploads(Arc<Mutex<HashMap<AssetId<Image>, Upload>>>);
impl DisplayUploads {
    pub fn publish(&self, handle: &Handle<Image>, image: &Image) {
        let _measure = crate::performance::span(10);
        assert_eq!(
            image.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb
        );
        let Some(bytes) = &image.data else { return };
        let mut pending = self.0.lock().unwrap();
        let upload = pending.entry(handle.id()).or_insert_with(|| Upload {
            size: image.texture_descriptor.size,
            bytes: Vec::new(),
            staging: Vec::new(),
            dirty: None,
        });
        upload.publish(image.texture_descriptor.size, bytes);
    }
}
pub struct DisplayUploadPlugin;
impl Plugin for DisplayUploadPlugin {
    fn build(&self, app: &mut App) {
        let uploads = DisplayUploads::default();
        app.insert_resource(uploads.clone())
            .add_systems(PostUpdate, retire);
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.insert_resource(uploads).add_systems(
                Render,
                flush
                    .in_set(RenderSystems::PrepareAssets)
                    .after(prepare_assets::<GpuImage>),
            );
        }
    }
}
fn retire(mut events: MessageReader<AssetEvent<Image>>, uploads: Res<DisplayUploads>) {
    for event in events.read() {
        if let AssetEvent::Removed { id } = event {
            uploads.0.lock().unwrap().remove(id);
        }
    }
}
fn flush(
    uploads: Res<DisplayUploads>,
    images: Res<RenderAssets<GpuImage>>,
    queue: Res<RenderQueue>,
    mut work: Local<Vec<Work>>,
) {
    let _measure = crate::performance::span(11);
    {
        let mut pending = uploads.0.lock().unwrap();
        for (&id, upload) in pending.iter_mut() {
            let Some(gpu) = images.get(id) else {
                continue;
            };
            let Some((rect, bytes)) = upload.prepare(gpu.size) else {
                continue;
            };
            work.push(Work { id, rect, bytes });
        }
    }
    // GPU queue calls happen outside the producer lock. A concurrent publish
    // remains dirty for the next flush and cannot be overwritten by this batch.
    for job in work.drain(..) {
        let gpu = images.get(job.id).unwrap();
        if crate::performance::enabled() {
            crate::performance::UPLOAD_BYTES
                .fetch_add(job.bytes.len() as u64, std::sync::atomic::Ordering::Relaxed);
            crate::performance::UPLOAD_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &gpu.texture,
                mip_level: 0,
                origin: Origin3d {
                    x: job.rect.x,
                    y: job.rect.y,
                    z: 0,
                },
                aspect: default(),
            },
            &job.bytes,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(job.rect.width() * 4),
                rows_per_image: Some(job.rect.height()),
            },
            Extent3d {
                width: job.rect.width(),
                height: job.rect.height(),
                depth_or_array_layers: 1,
            },
        );
        if let Some(upload) = uploads.0.lock().unwrap().get_mut(&job.id) {
            upload.staging = job.bytes;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn state(size: Extent3d) -> Upload {
        Upload {
            size,
            bytes: Vec::new(),
            staging: Vec::new(),
            dirty: None,
        }
    }
    fn apply(gpu: &mut [u8], size: Extent3d, rect: Rect, bytes: &[u8]) {
        for row in 0..rect.height() as usize {
            let target = ((rect.y as usize + row) * size.width as usize + rect.x as usize) * 4;
            let width = rect.width() as usize * 4;
            gpu[target..target + width].copy_from_slice(&bytes[row * width..(row + 1) * width]);
        }
    }
    #[test]
    fn interleaved_publication_and_flush_reconstruct_latest_pixels_and_reuse_storage() {
        let size = Extent3d {
            width: 16,
            height: 10,
            depth_or_array_layers: 1,
        };
        let mut upload = state(size);
        let mut data = vec![0; 16 * 10 * 4];
        upload.publish(size, &data);
        let (rect, bytes) = upload.prepare(size).unwrap();
        let mut gpu = data.clone();
        apply(&mut gpu, size, rect, &bytes);
        let capacity = bytes.capacity();
        upload.staging = bytes;
        // A flush has taken the first region when a second publication arrives.
        data[4 * 17] = 25;
        upload.publish(size, &data);
        let (first, bytes) = upload.prepare(size).unwrap();
        data[4 * 91 + 2] = 200;
        upload.publish(size, &data);
        apply(&mut gpu, size, first, &bytes);
        upload.staging = bytes;
        let (second, bytes) = upload.prepare(size).unwrap();
        apply(&mut gpu, size, second, &bytes);
        upload.staging = bytes;
        assert_eq!(gpu, data);
        assert!(upload.staging.capacity() >= capacity);
        upload.publish(size, &data);
        assert!(upload.prepare(size).is_none());
        // Two changes, including a reverted pixel, before the next flush.
        data[4] = 100;
        upload.publish(size, &data);
        data[4] = 0;
        data[8] = 55;
        upload.publish(size, &data);
        let (rect, bytes) = upload.prepare(size).unwrap();
        apply(&mut gpu, size, rect, &bytes);
        assert_eq!(gpu, data);
    }
    #[test]
    fn shrinking_and_delayed_gpu_preparation_keep_full_latest_image() {
        let large = Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        };
        let small = Extent3d {
            width: 2,
            height: 3,
            depth_or_array_layers: 1,
        };
        let mut upload = state(large);
        upload.publish(large, &[10; 256]);
        upload.publish(small, &[27; 24]);
        assert!(upload.prepare(large).is_none());
        assert_eq!(upload.dirty, Some(Rect::full(small)));
        let (rect, bytes) = upload.prepare(small).unwrap();
        assert_eq!(rect, Rect::full(small));
        assert_eq!(bytes, vec![27; 24]);
    }
    #[test]
    fn dirty_region_contains_all_changed_pixels_and_unchanged_images_upload_nothing() {
        let size = Extent3d {
            width: 8,
            height: 6,
            depth_or_array_layers: 1,
        };
        let old = vec![0; 8 * 6 * 4];
        let mut new = old.clone();
        assert_eq!(changed_rect(&old, &new, size), None);
        new[(2 * 8 + 3) * 4] = 1;
        new[(4 * 8 + 6) * 4 + 3] = 255;
        assert_eq!(
            changed_rect(&old, &new, size),
            Some(Rect {
                x: 3,
                y: 2,
                right: 7,
                bottom: 5
            })
        );
        assert_eq!(changed_rect(&[], &new, size), Some(Rect::full(size)));
    }
    #[test]
    fn latest_pixels_replace_pending_updates_without_replacing_the_asset() {
        use bevy::{
            asset::{AssetApp, AssetEvent, AssetPlugin, RenderAssetUsages},
            render::render_resource::TextureDimension,
        };
        let mut app = App::new();
        app.add_plugins((bevy::app::TaskPoolPlugin::default(), AssetPlugin::default()))
            .init_asset::<Image>();
        let h = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::new_fill(
                Extent3d {
                    width: 4,
                    height: 2,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[0, 0, 0, 255],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            ));
        app.update();
        app.world_mut()
            .resource_mut::<Messages<AssetEvent<Image>>>()
            .drain()
            .for_each(drop);
        let uploads = DisplayUploads::default();
        for n in 0..100 {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            let image = images.get_mut_untracked(&h).unwrap();
            image.data.as_mut().unwrap()[0] = n;
            uploads.publish(&h, image);
        }
        let pending = uploads.0.lock().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[&h.id()].bytes[0], 99);
        assert_eq!(pending[&h.id()].bytes.len(), 32);
        drop(pending);
        app.update();
        assert!(
            !app.world_mut()
                .resource_mut::<Messages<AssetEvent<Image>>>()
                .drain()
                .any(|e| matches!(e, AssetEvent::Modified { .. }))
        );
    }
}
