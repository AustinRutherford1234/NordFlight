use crate::{
    physics::{Flight, Vehicle},
    world::frame,
};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
#[derive(Resource)]
pub struct Navball {
    pub image: Handle<Image>,
    // Geometry of the projected sphere never changes; cache square roots and
    // shading once rather than rebuilding them on every instrument update.
    pixels: Vec<Option<(bevy::math::DVec3, f32)>>,
}
const SIZE: usize = 192;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stationary_ball_does_not_queue_repeated_texture_uploads() {
        use bevy::asset::{AssetApp, AssetEvent, AssetPlugin};
        let mut app = App::new();
        app.add_plugins((bevy::app::TaskPoolPlugin::default(), AssetPlugin::default()))
            .init_asset::<Image>()
            .insert_resource(Flight::runway())
            .insert_resource(Time::<()>::default())
            .add_systems(Startup, setup)
            .add_systems(Update, update);
        let advance = |app: &mut App| {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(60));
            app.update();
            app.world_mut()
                .resource_mut::<Messages<AssetEvent<Image>>>()
                .drain()
                .filter(|e| matches!(e, AssetEvent::Modified { .. }))
                .count()
        };
        assert!(advance(&mut app) > 0, "initial texture must be drawn");
        assert_eq!(advance(&mut app), 0, "unchanged ball triggered an upload");
        app.world_mut().resource_mut::<Flight>().orientation *=
            bevy::math::DQuat::from_rotation_z(0.1);
        assert!(advance(&mut app) > 0, "changed attitude must redraw");
    }
}
#[derive(Component)]
pub struct Overlay;
pub fn setup(mut c: Commands, mut images: ResMut<Assets<Image>>) {
    let image = images.add(Image::new_fill(
        Extent3d {
            width: SIZE as u32,
            height: SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    c.spawn((
        ImageNode::new(image.clone()),
        Overlay,
        Node {
            position_type: PositionType::Absolute,
            bottom: px(34),
            left: percent(50),
            margin: UiRect::left(px(-96)),
            width: px(192),
            height: px(192),
            ..default()
        },
        ZIndex(30),
    ));
    c.spawn((
        Text::new("SURFACE / ATTITUDE"),
        Overlay,
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.75, 0.93, 0.94)),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(16),
            left: percent(50),
            margin: UiRect::left(px(-155)),
            ..default()
        },
        ZIndex(30),
        NavballLabel,
    ));
    let pixels = (0..SIZE * SIZE)
        .map(|i| {
            let sx = (i % SIZE) as f64 - 95.5;
            let sy = 95.5 - (i / SIZE) as f64;
            let r2 = (sx * sx + sy * sy) / (88.0 * 88.0);
            (r2 <= 1.0).then(|| {
                let z = (1.0 - r2).sqrt();
                (
                    bevy::math::DVec3::new(sx / 88.0, sy / 88.0, -z),
                    (0.55 + 0.45 * z) as f32,
                )
            })
        })
        .collect();
    c.insert_resource(Navball { image, pixels });
}
#[derive(Component)]
pub struct NavballLabel;
pub fn update(
    f: Res<Flight>,
    ball: Res<Navball>,
    mut images: ResMut<Assets<Image>>,
    uploads: Option<Res<crate::display_upload::DisplayUploads>>,
    mut label: Query<&mut Text, With<NavballLabel>>,
    time: Res<Time>,
    mut last: Local<f32>,
    mut previous: Local<Option<(bevy::math::DQuat, bevy::math::DVec3)>>,
) {
    if time.elapsed_secs() - *last < 0.05 {
        return;
    }
    *last = time.elapsed_secs();
    let attitude = bevy::math::DQuat::from_rotation_y(-184.59_f64.to_radians())
        * frame(&f).conjugate()
        * f.orientation;
    let velocity = if f.vehicle == Vehicle::Spacecraft {
        crate::space::relative(&f).1
    } else {
        f.velocity
    };
    let body_velocity = f.orientation.conjugate() * velocity;
    if previous.as_ref().is_some_and(|(q, v)| {
        q.dot(attitude).abs() > 0.999999999 && (*v - body_velocity).length_squared() < 1e-8
    }) {
        return;
    }
    // Requesting mutable access queues a GPU upload even if no pixel changes.
    let image = if uploads.is_some() {
        images.get_mut_untracked(&ball.image)
    } else {
        images.get_mut(&ball.image)
    };
    let Some(image) = image else {
        return;
    };
    let Some(data) = image.data.as_mut() else {
        return;
    };
    *previous = Some((attitude, body_velocity));
    for mut text in &mut label {
        text.0 = format!(
            "{}  HDG {:03.0}°  P {:+.0}°  R {:+.0}°",
            if f.vehicle == Vehicle::Spacecraft {
                "ORBIT"
            } else {
                "SURFACE"
            },
            f.heading(),
            f.pitch().to_degrees(),
            f.roll().to_degrees()
        );
    }
    for y in 0..SIZE {
        for x in 0..SIZE {
            let sx = (x as f64 - 95.5) / 88.0;
            let sy = (95.5 - y as f64) / 88.0;
            let r2 = sx * sx + sy * sy;
            let rgba = if r2 > 1.15 {
                [0, 0, 0, 0]
            } else if r2 > 1.0 {
                [75, 91, 104, 255]
            } else {
                let (sphere, shade) = ball.pixels[y * SIZE + x].unwrap();
                let local = attitude * sphere;
                let latitude = local.y.clamp(-1.0, 1.0).asin().to_degrees();
                let longitude = local.x.atan2(-local.z).to_degrees();
                let grid = latitude.abs() < 0.8
                    || (latitude / 15.0 - (latitude / 15.0).round()).abs() < 0.035
                    || (longitude / 30.0 - (longitude / 30.0).round()).abs() < 0.018;
                let color = if grid {
                    [185.0, 204.0, 210.0]
                } else if latitude >= 0.0 {
                    [37.0, 109.0, 174.0]
                } else {
                    [136.0, 79.0, 43.0]
                };
                [
                    (color[0] * shade) as u8,
                    (color[1] * shade) as u8,
                    (color[2] * shade) as u8,
                    255,
                ]
            };
            let n = (y * SIZE + x) * 4;
            data[n..n + 4].copy_from_slice(&rgba);
        }
    }
    let mut pixel = |x: i32, y: i32, c: [u8; 4]| {
        if x >= 0 && y >= 0 && x < SIZE as i32 && y < SIZE as i32 {
            let n = (y as usize * SIZE + x as usize) * 4;
            data[n..n + 4].copy_from_slice(&c);
        }
    };
    // Heading numerals live on the sphere rather than on the bezel.
    for (heading, label) in [
        (0.0_f64, "000"),
        (90.0, "090"),
        (180.0, "180"),
        (270.0, "270"),
    ] {
        let a = heading.to_radians();
        let direction = attitude.conjugate() * bevy::math::DVec3::new(a.sin(), 0.0, -a.cos());
        if direction.z < -0.25 {
            let x = (95.5 + 88.0 * direction.x) as i32 - 10;
            let y = (95.5 - 88.0 * direction.y) as i32 + 4;
            for (index, glyph) in label.bytes().enumerate() {
                let rows = match glyph {
                    b'0' => [7u8, 5, 5, 5, 7],
                    b'1' => [2, 6, 2, 2, 7],
                    b'2' => [7, 1, 7, 4, 7],
                    b'7' => [7, 1, 2, 2, 2],
                    b'8' => [7, 5, 7, 5, 7],
                    b'9' => [7, 5, 7, 1, 7],
                    _ => [0; 5],
                };
                for (row, bits) in rows.iter().enumerate() {
                    for col in 0..3 {
                        if bits & (1 << (2 - col)) != 0 {
                            for dy in 0..2 {
                                for dx in 0..2 {
                                    pixel(
                                        x + index as i32 * 8 + col * 2 + dx,
                                        y + row as i32 * 2 + dy,
                                        [240, 238, 218, 255],
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // Project direction onto the visible hemisphere; suppress unstable markers at rest.
    if body_velocity.length() > 0.5 {
        let pro = body_velocity.normalize();
        for (direction, retro) in [(pro, false), (-pro, true)] {
            if direction.z < -0.02 {
                let cx = (95.5 + 88.0 * direction.x) as i32;
                let cy = (95.5 - 88.0 * direction.y) as i32;
                for dy in -14i32..=14 {
                    for dx in -14i32..=14 {
                        let d = dx * dx + dy * dy;
                        if (35..=55).contains(&d)
                            || (retro && (dx.abs() == dy.abs()) && d < 35)
                            || (!retro && d < 4)
                            || (!retro && dx.abs() >= 8 && dx.abs() <= 14 && dy.abs() <= 1)
                            || (!retro && dx.abs() <= 1 && (-14..=-8).contains(&dy))
                        {
                            pixel(cx + dx, cy + dy, [230, 246, 80, 255]);
                        }
                    }
                }
            }
        }
    }
    if f.vehicle == Vehicle::Spacecraft {
        let orbit = crate::orbit::Orbit::from_flight(&f);
        let inverse = f.orientation.conjugate();
        let radial = inverse * f.up();
        let normal = inverse * orbit.angular_momentum.normalize_or_zero();
        for (direction, triangle) in [
            (radial, false),
            (-radial, false),
            (normal, true),
            (-normal, true),
        ] {
            if direction.z < -0.03 {
                let cx = (95.5 + 88.0 * direction.x) as i32;
                let cy = (95.5 - 88.0 * direction.y) as i32;
                for dy in -7i32..=7 {
                    for dx in -7i32..=7 {
                        let draw = if triangle {
                            dy >= -6 && dy <= 6 && (dx.abs() == (dy + 6) / 2 || dy == 6)
                        } else {
                            (dx.abs() <= 1 || dy.abs() <= 1) && dx * dx + dy * dy > 9
                        };
                        if draw {
                            pixel(
                                cx + dx,
                                cy + dy,
                                if triangle {
                                    [233, 129, 238, 255]
                                } else {
                                    [86, 228, 243, 255]
                                },
                            );
                        }
                    }
                }
            }
        }
    }
    for dx in -25i32..=25 {
        if dx.abs() > 8 || dx.abs() < 3 {
            pixel(96 + dx, 96, [255, 190, 54, 255]);
            pixel(96 + dx, 97, [255, 190, 54, 255]);
        }
    }
    for dy in 0..8 {
        pixel(88, 96 + dy, [255, 190, 54, 255]);
        pixel(104, 96 + dy, [255, 190, 54, 255]);
    }
    if let Some(uploads) = uploads {
        uploads.publish(&ball.image, image);
    }
}
