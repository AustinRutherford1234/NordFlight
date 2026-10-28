//! Cockpit G-LOC vision and the optional player input consequence. This uses
//! the existing camera's color grading plus four UI quads, never another view.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    render::{
        render_resource::{Extent3d, TextureDimension},
        view::ColorGrading,
    },
};
#[derive(Component)]
pub(super) struct VisionLayer(u8);
#[derive(Component)]
pub(super) struct BaseSaturation(f32);
#[derive(Resource)]
pub(super) struct HeldInput {
    throttle: f64,
    autopilot: bool,
}
impl Default for HeldInput {
    fn default() -> Self {
        Self {
            throttle: 0.25,
            autopilot: false,
        }
    }
}
pub(super) fn setup(
    mut c: Commands,
    mut images: ResMut<Assets<Image>>,
    cameras: Query<(Entity, Option<&ColorGrading>), With<FlightCamera>>,
) {
    let mut pixels = Vec::with_capacity(128 * 128 * 4);
    for y in 0..128 {
        for x in 0..128 {
            let p = Vec2::new(x as f32 / 127. * 2. - 1., y as f32 / 127. * 2. - 1.);
            let r = p.length();
            let u = ((r - 0.18) / 1.10).clamp(0., 1.);
            let a = u * u * (3. - 2. * u);
            pixels.extend([255, 255, 255, (a * 255.) as u8]);
        }
    }
    let image = images.add(Image::new(
        Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    for (camera, grading) in &cameras {
        let grading = grading.cloned().unwrap_or_default();
        c.entity(camera)
            .insert((BaseSaturation(grading.global.post_saturation), grading));
        for layer in 0..4 {
            let e = c
                .spawn((
                    VisionLayer(layer),
                    UiTargetCamera(camera),
                    GlobalZIndex(76 + layer as i32),
                    bevy::ui::FocusPolicy::Pass,
                    bevy::picking::Pickable::IGNORE,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        width: percent(100),
                        height: percent(100),
                        display: Display::None,
                        ..default()
                    },
                ))
                .id();
            if layer == 2 {
                c.entity(e).insert(ImageNode::new(image.clone()));
            } else {
                c.entity(e).insert(BackgroundColor(Color::NONE));
            }
        }
    }
}
pub(super) fn control_loss(
    crew: Res<Crew>,
    states: Res<Reactions>,
    view: Res<ViewState>,
    menu: Res<crate::start_screen::Menu>,
    mut f: ResMut<Flight>,
    mut held: ResMut<HeldInput>,
    combat: Option<ResMut<Combat>>,
) {
    if !crew.control_loss
        || menu.open
        || f.paused
        || !supported(&f)
        || states.generation != view.generation
        || states.states[0].face.lost
    {
        held.throttle = f.throttle;
        held.autopilot = f.autopilot;
        return;
    }
    let authority = states.states[0].conscious.authority();
    f.controls *= authority as f64;
    if authority < 0.05 {
        f.throttle = held.throttle;
        f.autopilot = held.autopilot;
        f.brakes = false;
        if let Some(mut combat) = combat {
            combat.stop_gun();
        }
    } else {
        held.throttle = f.throttle;
        held.autopilot = f.autopilot;
    }
}
pub(super) fn draw(
    crew: Res<Crew>,
    states: Res<Reactions>,
    tracks: Res<Performances>,
    view: Res<ViewState>,
    menu: Res<crate::start_screen::Menu>,
    f: Res<Flight>,
    mut cameras: Query<(&BaseSaturation, &mut ColorGrading), With<FlightCamera>>,
    mut layers: Query<(
        &VisionLayer,
        &mut Node,
        Option<&mut BackgroundColor>,
        Option<&mut ImageNode>,
    )>,
) {
    let active = crew.cockpit_g_effects && !menu.open
        && view.index == 3
        && !view.map
        && !view.following()
        && supported(&f)
        && !states.states[0].face.lost;
    let state = if demo_capture() && matches!(crew.preview, Preview::Faint(_)) {
        tracks.preview_state
    } else {
        states.states[0].conscious
    };
    let v = if active {
        state.vision()
    } else {
        conscious::Vision::default()
    };
    for (base, mut grading) in &mut cameras {
        let target = base.0 * (1. - v.grey);
        if (grading.global.post_saturation - target).abs() > 0.0001
            || (!active && grading.global.post_saturation != base.0)
        {
            grading.global.post_saturation = target;
        }
    }
    for (tag, mut node, background, image) in &mut layers {
        let color = match tag.0 {
            0 => Color::srgba(0.20, 0.20, 0.20, v.grey * 0.13),
            1 => Color::srgba(0.80, 0.002, 0.0, v.red * 0.72),
            2 => Color::srgba(0., 0., 0., v.edge * 0.94),
            _ => Color::srgba(0., 0., 0., v.black),
        };
        let show = color.alpha() > 0.0001;
        let display = if show { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
        if let Some(mut bg) = background {
            bg.set_if_neq(BackgroundColor(color));
        }
        if let Some(mut image) = image {
            if image.color != color {
                image.color = color;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> App {
        let mut app = App::new();
        let mut f = Flight::approach();
        f.aircraft = Aircraft::F22;
        f.paused = false;
        app.insert_resource(f)
            .init_resource::<Crew>()
            .init_resource::<Reactions>()
            .init_resource::<Performances>()
            .init_resource::<ViewState>()
            .init_resource::<crate::start_screen::Menu>()
            .init_resource::<HeldInput>();
        app.world_mut().resource_mut::<Reactions>().generation = 0;
        app.world_mut().resource_mut::<Reactions>().states[0].conscious =
            conscious::Preview::Out.snapshot(2.);
        app
    }
    #[test]
    fn control_consequence_is_opt_in_and_holds_throttle_without_blocking_camera_or_pause() {
        let mut app = app();
        app.add_systems(Update, control_loss);
        app.world_mut().resource_mut::<Crew>().cockpit_g_effects = false;
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.controls = bevy::math::DVec3::ONE;
            f.throttle = 0.8;
        }
        app.update();
        assert_eq!(
            app.world().resource::<Flight>().controls,
            bevy::math::DVec3::ONE
        );
        app.world_mut().resource_mut::<Crew>().control_loss = true;
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.controls = bevy::math::DVec3::ONE;
            f.throttle = 0.;
        }
        app.update();
        assert_eq!(
            app.world().resource::<Flight>().controls,
            bevy::math::DVec3::ZERO
        );
        assert_eq!(app.world().resource::<Flight>().throttle, 0.8);
        assert!(!app.world().resource::<Flight>().paused);
        app.world_mut().resource_mut::<Crew>().control_loss = false;
        app.world_mut().resource_mut::<Flight>().controls = bevy::math::DVec3::ONE;
        app.update();
        assert_eq!(
            app.world().resource::<Flight>().controls,
            bevy::math::DVec3::ONE
        );
    }
    #[test]
    fn blackout_is_opaque_in_cockpit_and_grading_restores_when_switching_views() {
        let mut app = app();
        app.add_systems(Update, draw);
        let cam = app
            .world_mut()
            .spawn((FlightCamera, BaseSaturation(1.), ColorGrading::default()))
            .id();
        let overlay = app
            .world_mut()
            .spawn((
                VisionLayer(3),
                Node::default(),
                BackgroundColor(Color::NONE),
            ))
            .id();
        app.world_mut().resource_mut::<ViewState>().index = 3;
        app.update();
        assert_eq!(
            app.world()
                .get::<BackgroundColor>(overlay)
                .unwrap()
                .0
                .alpha(),
            1.
        );
        assert!(
            app.world()
                .get::<ColorGrading>(cam)
                .unwrap()
                .global
                .post_saturation
                < 0.1
        );
        app.world_mut().resource_mut::<Crew>().cockpit_g_effects = false;
        app.update();
        assert_eq!(app.world().get::<Node>(overlay).unwrap().display,Display::None);
        assert_eq!(app.world().get::<ColorGrading>(cam).unwrap().global.post_saturation,1.);
        assert!(matches!(app.world().resource::<Reactions>().states[0].conscious.phase,conscious::Phase::Unconscious));
        app.world_mut().resource_mut::<Crew>().cockpit_g_effects = true;
        app.update();
        assert_eq!(app.world().get::<BackgroundColor>(overlay).unwrap().0.alpha(),1.);
        app.world_mut().resource_mut::<Crew>().editor = true;
        app.update();
        assert_eq!(
            app.world()
                .get::<BackgroundColor>(overlay)
                .unwrap()
                .0
                .alpha(),
            1.
        );
        app.world_mut().resource_mut::<Reactions>().states[0].conscious = default();
        app.world_mut().resource_mut::<Crew>().preview = Preview::Faint(conscious::Preview::Out);
        app.update();
        assert_eq!(
            app.world().get::<Node>(overlay).unwrap().display,
            Display::None
        );
        app.world_mut().resource_mut::<ViewState>().index = 0;
        app.update();
        assert_eq!(
            app.world().get::<Node>(overlay).unwrap().display,
            Display::None
        );
        assert_eq!(
            app.world()
                .get::<ColorGrading>(cam)
                .unwrap()
                .global
                .post_saturation,
            1.
        );
    }
}
