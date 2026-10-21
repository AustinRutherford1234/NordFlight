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
    #[cfg(test)]
    staging: Vec<u8>,
    dirty: Option<Rect>,
    revision: u64,
}
impl Upload {
    fn publish_rows(&mut self, size: Extent3d, bytes: &[u8], top: u32, bottom: u32) {
        assert!(top < bottom && bottom <= size.height);
        assert_eq!(size.depth_or_array_layers, 1);
        assert_eq!(bytes.len(), size.width as usize * size.height as usize * 4);
        if self.size != size || self.bytes.len() != bytes.len() {
            self.publish(size, bytes);
            return;
        }
        let stride=size.width as usize*4;
        let range=top as usize*stride..bottom as usize*stride;
        self.bytes[range.clone()].copy_from_slice(&bytes[range]);
        let rect=Rect {x:0,y:top,right:size.width,bottom};
        self.dirty=Some(self.dirty.map_or(rect,|old|old.union(rect)));
    }
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
    #[cfg(test)]
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
struct Packet {
    size: Extent3d,
    rect: Rect,
    bytes: Vec<u8>,
    revision: u64,
}
#[derive(Default)]
struct Handoff {
    pending: HashMap<AssetId<Image>, Packet>,
    recycled: HashMap<AssetId<Image>, Vec<u8>>,
    live: std::collections::HashSet<AssetId<Image>>,
}
impl Handoff {
    fn recycle(&mut self, id:AssetId<Image>,bytes:Vec<u8>) {
        if self.live.contains(&id) && self.recycled.get(&id).is_none_or(|v|v.capacity()<bytes.capacity()) {self.recycled.insert(id,bytes);}
    }
    fn put(&mut self,id:AssetId<Image>,mut packet:Packet) {
        if !self.live.contains(&id) {return;}
        if let Some(mut previous)=self.pending.remove(&id) {
            if previous.revision>packet.revision {std::mem::swap(&mut previous,&mut packet);}
            if previous.size==packet.size {packet.rect=packet.rect.union(previous.rect);}
            self.recycle(id,previous.bytes);
        }
        self.pending.insert(id,packet);
    }
}
#[derive(Resource, Clone, Default)]
pub struct DisplayUploads {
    // Only producers use this mutex; render never blocks their pixel work.
    producer: Arc<Mutex<HashMap<AssetId<Image>,Upload>>>,
    handoff: Arc<Mutex<Handoff>>,
}
impl DisplayUploads {
    fn publish_inner(&self,handle:&Handle<Image>,image:&Image,rows:Option<(u32,u32)>) {
        assert_eq!(image.texture_descriptor.format,TextureFormat::Rgba8UnormSrgb);
        let Some(bytes)=&image.data else {return};
        let id=handle.id();
        let mut producers=self.producer.lock().unwrap();
        let upload=producers.entry(id).or_insert_with(||Upload {size:image.texture_descriptor.size,bytes:vec![],#[cfg(test)] staging:vec![],dirty:None,revision:0});
        if let Some((top,bottom))=rows {upload.publish_rows(image.texture_descriptor.size,bytes,top,bottom);} else {upload.publish(image.texture_descriptor.size,bytes);}
        let Some(rect)=upload.dirty.take() else {return};
        upload.revision=upload.revision.wrapping_add(1);
        let mut snapshot={let mut q=self.handoff.lock().unwrap();q.live.insert(id);q.recycled.remove(&id).unwrap_or_default()};
        // Copy outside the shared queue lock; only complete owned snapshots cross.
        snapshot.clear();snapshot.extend_from_slice(&upload.bytes);
        self.handoff.lock().unwrap().put(id,Packet {size:upload.size,rect,bytes:snapshot,revision:upload.revision});
    }
    /// Caller has retained unchanged rows and supplies the complete changed span.
    /// Pending regions are unioned, including updates not yet flushed to the GPU.
    pub fn publish_rows(&self, handle:&Handle<Image>, image:&Image, top:u32, bottom:u32) {
        self.publish_inner(handle,image,Some((top,bottom)));
    }
    pub fn publish(&self, handle: &Handle<Image>, image: &Image) {
        let _measure = crate::performance::span(10);
        self.publish_inner(handle,image,None);
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
            uploads.producer.lock().unwrap().remove(id);
            let mut q=uploads.handoff.lock().unwrap();q.live.remove(id);q.pending.remove(id);q.recycled.remove(id);
        }
    }
}
fn flush(
    uploads: Res<DisplayUploads>,
    images: Res<RenderAssets<GpuImage>>,
    queue: Res<RenderQueue>,
    mut work: Local<Vec<(AssetId<Image>,Packet)>>,
    mut staging: Local<Vec<u8>>,
) {
    let _measure = crate::performance::span(11);
    {
        let mut q = uploads.handoff.lock().unwrap();
        work.extend(q.pending.drain());
    }
    // GPU queue calls happen outside the producer lock. A concurrent publish
    // remains dirty for the next flush and cannot be overwritten by this batch.
    for (id,job) in work.drain(..) {
        let Some(gpu)=images.get(id).filter(|gpu|gpu.size==job.size) else {uploads.handoff.lock().unwrap().put(id,job);continue;};
        staging.clear();
        let stride=job.size.width as usize*4;
        for y in job.rect.y..job.rect.bottom {let start=y as usize*stride+job.rect.x as usize*4;staging.extend_from_slice(&job.bytes[start..start+job.rect.width() as usize*4]);}
        if crate::performance::enabled() {
            crate::performance::UPLOAD_BYTES
                .fetch_add(staging.len() as u64, std::sync::atomic::Ordering::Relaxed);
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
            &staging,
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
        uploads.handoff.lock().unwrap().recycle(id,job.bytes);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn consumer_retry_merges_newer_revision_and_retirement_rejects_old_work() {
        let id=Handle::<Image>::default().id();
        let size=Extent3d {width:4,height:4,depth_or_array_layers:1};
        let mut q=Handoff::default();q.live.insert(id);
        q.put(id,Packet{size,rect:Rect{x:0,y:0,right:1,bottom:1},bytes:vec![1;64],revision:1});
        let in_flight=q.pending.remove(&id).unwrap();
        q.put(id,Packet{size,rect:Rect{x:3,y:3,right:4,bottom:4},bytes:vec![2;64],revision:2});
        q.put(id,in_flight);
        let latest=q.pending.remove(&id).unwrap();
        assert_eq!(latest.revision,2);assert_eq!(latest.rect,Rect::full(size));
        assert!(latest.bytes.iter().all(|v|*v==2));
        q.live.remove(&id);q.recycled.remove(&id);
        q.put(id,latest);assert!(q.pending.is_empty());assert!(q.recycled.is_empty());
    }
    #[test]
    fn retry_before_resize_cannot_restore_obsolete_bounds_or_pixels() {
        let id=Handle::<Image>::default().id();let mut q=Handoff::default();q.live.insert(id);
        let large=Extent3d {width:8,height:8,depth_or_array_layers:1};
        let small=Extent3d {width:2,height:2,depth_or_array_layers:1};
        q.put(id,Packet{size:small,rect:Rect::full(small),bytes:vec![9;16],revision:2});
        q.put(id,Packet{size:large,rect:Rect::full(large),bytes:vec![1;256],revision:1});
        let latest=&q.pending[&id];assert_eq!(latest.size,small);assert_eq!(latest.rect,Rect::full(small));
        assert_eq!(latest.bytes,vec![9;16]);
    }
    #[test]
    fn publishing_remains_possible_while_consumer_owns_previous_snapshot() {
        let uploads=DisplayUploads::default();let h=Handle::<Image>::default();
        let image=Image::new_fill(Extent3d{width:4,height:4,depth_or_array_layers:1},
            bevy::render::render_resource::TextureDimension::D2,&[0,0,0,255],TextureFormat::Rgba8UnormSrgb,default());
        uploads.publish(&h,&image);
        let snapshot=uploads.handoff.lock().unwrap().pending.remove(&h.id()).unwrap();
        let writer=uploads.clone();let handle=h.clone();
        std::thread::spawn(move||{let mut image=image;image.data.as_mut().unwrap()[0]=11;writer.publish(&handle,&image);}).join().unwrap();
        assert_eq!(snapshot.bytes[0],0);
        assert_eq!(uploads.handoff.lock().unwrap().pending[&h.id()].bytes[0],11);
    }
    #[test]
    fn reported_rows_union_with_pending_updates_and_survive_missing_gpu() {
        let size=Extent3d {width:8,height:6,depth_or_array_layers:1};
        let mut upload=state(size);
        let mut bytes=vec![0;8*6*4];
        upload.publish(size,&bytes);upload.prepare(size);
        bytes[32..64].fill(7);upload.publish_rows(size,&bytes,1,2);
        bytes[128..160].fill(9);upload.publish_rows(size,&bytes,4,5);
        assert!(upload.prepare(Extent3d {width:1,..size}).is_none());
        let (rect,data)=upload.prepare(size).unwrap();
        assert_eq!(rect,Rect {x:0,y:1,right:8,bottom:5});
        assert_eq!(data,bytes[32..160]);
        assert_eq!(upload.bytes,bytes);
    }
    fn state(size: Extent3d) -> Upload {
        Upload {
            size,
            bytes: Vec::new(),
            staging: Vec::new(),
            dirty: None,
            revision: 0,
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
        let q = uploads.handoff.lock().unwrap();
        let pending = &q.pending;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[&h.id()].bytes[0], 99);
        assert_eq!(pending[&h.id()].bytes.len(), 32);
        drop(q);
        app.update();
        assert!(
            !app.world_mut()
                .resource_mut::<Messages<AssetEvent<Image>>>()
                .drain()
                .any(|e| matches!(e, AssetEvent::Modified { .. }))
        );
    }
}
