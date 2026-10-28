//! Size the local portrait to its displayed pixels, retaining supersampling.
use super::*;

#[derive(Component)]
pub(super) struct PortraitSurface;

fn target_side(displayed: f32) -> u32 {
    // 1.5x linear supersampling plus the existing 4x MSAA. Quantization keeps
    // dragging/resizing from reallocating the target for every pixel moved.
    ((displayed * 1.5).clamp(320., 640.) / 64.).ceil() as u32 * 64
}

pub(super) fn resize_target(
    target: Res<PortraitImage>,
    mut images: ResMut<Assets<Image>>,
    surfaces: Query<&ComputedNode, With<PortraitSurface>>,
    cameras: Query<&Camera, With<PortraitCamera>>,
) {
    if !cameras.iter().any(|c| c.is_active) {
        return;
    }
    // ComputedNode is in physical pixels, so this also respects Retina/UI
    // scaling. Use the preceding layout before this frame's camera update.
    let displayed = surfaces
        .iter()
        .map(|n| n.size().max_element())
        .fold(0., f32::max);
    if displayed <= 0. {
        return;
    }
    let reference = std::env::var_os("NORD_CREW_PORTRAIT_REFERENCE").is_some()
        || std::env::var_os("NORD_CREW_CAPTURE").is_some();
    let side = if reference {
        640
    } else {
        target_side(displayed)
    };
    if images
        .get(&target.0)
        .is_some_and(|image| image.size() != UVec2::splat(side))
    {
        images
            .get_mut(&target.0)
            .unwrap()
            .resize(bevy::render::render_resource::Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Resource, Default)]
    struct ImageChanges(usize);
    fn count_image_changes(images: Res<Assets<Image>>, mut changes: ResMut<ImageChanges>) {
        if images.is_changed() {
            changes.0 += 1;
        }
    }
    #[test]
    fn portrait_supersamples_actual_pixels_and_retains_full_size_for_large_panels() {
        assert_eq!(target_side(244.), 384);
        assert_eq!(target_side(290.), 448);
        assert_eq!(target_side(488.), 640);
        assert_eq!(target_side(520.), 640);
        for displayed in 1..=426 {
            assert!(target_side(displayed as f32) as f32 >= displayed as f32 * 1.5);
        }
    }
    #[test]
    fn stable_layout_does_not_reallocate_or_mark_image_modified() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>();
        app.init_resource::<ImageChanges>();
        let handle =
            app.world_mut()
                .resource_mut::<Assets<Image>>()
                .add(Image::new_target_texture(
                    640,
                    640,
                    TextureFormat::Rgba8Unorm,
                    Some(TextureFormat::Rgba8UnormSrgb),
                ));
        app.insert_resource(PortraitImage(handle.clone()));
        app.world_mut().spawn((PortraitCamera, Camera::default()));
        app.world_mut().spawn((
            PortraitSurface,
            ComputedNode {
                size: Vec2::new(244., 200.),
                ..default()
            },
        ));
        app.add_systems(Update, (resize_target, count_image_changes).chain());
        app.update();
        assert_eq!(
            app.world()
                .resource::<Assets<Image>>()
                .get(&handle)
                .unwrap()
                .size(),
            UVec2::splat(384)
        );
        assert_eq!(app.world().resource::<ImageChanges>().0, 1);
        app.update();
        assert_eq!(app.world().resource::<ImageChanges>().0, 1);
        let mut nodes = app
            .world_mut()
            .query_filtered::<&mut ComputedNode, With<PortraitSurface>>();
        nodes.single_mut(app.world_mut()).unwrap().size = Vec2::splat(480.);
        app.update();
        assert_eq!(app.world().resource::<ImageChanges>().0, 2);
        assert_eq!(
            app.world()
                .resource::<Assets<Image>>()
                .get(&handle)
                .unwrap()
                .size(),
            UVec2::splat(640)
        );
    }
}
