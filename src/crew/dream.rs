//! One-shot memory during the player's unconscious cockpit view.
use super::*;

#[derive(Component)]
pub(super) struct DreamPart(u8);

pub(super) fn setup(mut commands: Commands, cameras: Query<Entity, With<FlightCamera>>, assets: Res<AssetServer>) {
    for camera in &cameras {
        for index in 0..9 {
            let node = Node { position_type: PositionType::Absolute, display: Display::None,
                left: percent(0.), top: percent(0.), width: percent(100.), height: percent(100.), ..default() };
            let mut entity = commands.spawn((DreamPart(index), UiTargetCamera(camera), GlobalZIndex(500 + index as i32),
                bevy::ui::FocusPolicy::Pass, bevy::picking::Pickable::IGNORE, UiTransform::default(), node));
            if index == 0 { entity.insert(BackgroundColor(Color::BLACK)); }
            else { entity.insert(ImageNode::new(assets.load(if index >= 4 { "crew/dream/paw.png" } else { "crew/dream/dachshund.png" }))); }
        }
    }
}

fn dream_time(state: conscious::State, active: bool) -> Option<(f32, f32)> {
    if !active { return None; }
    match state.phase {
        conscious::Phase::Unconscious => Some((state.age.max(0.), 1.)),
        // Keep the hand during waking, but let the existing black vision layer
        // control the background and preserve the original recovery curve.
        conscious::Phase::Waking => Some((4. + state.age.max(0.), (1. - state.age / 3.2).clamp(0., 1.))),
        _ => None,
    }
}

fn picture_alpha(time: f32) -> f32 {
    (1. - ((time - 2.2) / 1.5).clamp(0., 1.)) * (time / 0.35).clamp(0., 1.)
}

fn memory_picture(breed: usize, slot: u8) -> Option<&'static str> {
    if breed != 7 { return None; }
    match slot {
        1 => Some("crew/dream/dachshund-beach.png"),
        2 => Some("crew/dream/dachshund-football.png"),
        3 => Some("crew/dream/dachshund-wedding.png"),
        _ => None,
    }
}

fn finger_offset(time: f32, finger: usize) -> f32 {
    // Four differently phased, small curls on top of a slower reaching motion.
    let ease = (time / 1.8).clamp(0., 1.);
    let curl = (1. - ((time - 2.0) / 1.8).clamp(0., 1.)) * 0.7 + 0.3;
    ease * curl * (time * 2.0 + finger as f32 * 0.75).sin() * 0.25
}

pub(super) fn draw(crew: Res<Crew>, states: Res<Reactions>, tracks: Res<Performances>,
    view: Res<ViewState>, menu: Res<crate::start_screen::Menu>, f: Res<Flight>,
    assets: Res<AssetServer>, bridge: Option<Res<crate::sound::Bridge>>,
    mut parts: Query<(&DreamPart, &mut Node, &mut UiTransform, Option<&mut BackgroundColor>, Option<&mut ImageNode>)>) {
    let active = crew.cockpit_g_effects && !menu.open && view.index == 3 && !view.map && !view.following()
        && supported(&f) && !states.states[0].face.lost;
    let state = if demo_capture() && matches!(crew.preview, Preview::Faint(_)) { tracks.preview_state } else { states.states[0].conscious };
    let mut timing = dream_time(state, active);
    // Freeze a specific pose for the existing native screenshot workflow.
    if demo_capture() && state.phase == conscious::Phase::Unconscious {
        if let Some(t) = std::env::var("NORD_DREAM_TIME").ok().and_then(|v|v.parse::<f32>().ok()).filter(|v|v.is_finite()) {
            timing = timing.map(|(_, alpha)| (t.max(0.), alpha));
        }
    }
    if let Some(bridge) = bridge { crate::sound::dream_music(&bridge, timing.map_or(0., |(_, alpha)| alpha), f.paused); }
    const NAMES: [&str; 10] = ["beagle", "shiba", "husky", "corgi", "labrador", "shepherd", "terrier", "dachshund", "bulldog", "mutt"];
    for (part, mut node, mut transform, background, image) in &mut parts {
        node.display = if timing.is_some() { Display::Flex } else { Display::None };
        let Some((time, darkness)) = timing else { continue; };
        if let Some(mut bg) = background { bg.0 = Color::srgba(0., 0., 0., if state.phase == conscious::Phase::Waking { 0. } else { 1. }); }
        let Some(mut image) = image else { continue; };
        if part.0 >= 4 {
            let reach = (time / 2.0).clamp(0., 1.);
            let sway = (time.min(4.) * 1.3).sin() * 0.7;
            let left = 35. + sway;
            let top = 64. - 18. * reach;
            // Keep the complete hand as a backing layer, with four separately
            // moving tip overlays. The backing prevents gaps at their seams.
            if part.0 == 4 {
                image.rect = None;
                node.left = percent(left); node.top = percent(top);
                node.width = percent(30.); node.height = percent(65.);
                transform.rotation = bevy::math::Rot2::IDENTITY;
            } else {
                let finger = (part.0 - 5) as usize;
                let x = finger as f32 * 256.;
                image.rect = Some(Rect::new(x, 0., x + 256., 640.));
                node.left = percent(left + finger as f32 * 7.5);
                node.top = percent(top + finger_offset(time, finger));
                node.width = percent(7.5); node.height = percent(65. * 640. / 1536.);
                transform.rotation = bevy::math::Rot2::radians(finger_offset(time, finger) * 0.006);
            }
            image.image = assets.load("crew/dream/paw.png");
            let coat = crew.appearances[crew.selected].coat;
            let tint = match coat { 3 => (0.35, 0.29, 0.25), 4 => (1., 0.91, 0.72), 6 => (0.67, 0.72, 0.77), _ => (1., 0.85, 0.72) };
            image.color = Color::srgba(tint.0,tint.1,tint.2,(time / 0.35).clamp(0., 1.) * darkness * if part.0 == 4 { 1. } else { 0.75 });
        } else {
            let index = part.0 as f32 - 2.;
            let drift = (time / 3.7).clamp(0., 1.);
            let scale = 1. - 0.48 * drift;
            node.width = percent(24. * scale); node.height = percent(34. * scale);
            node.left = percent(38. + index * (25. + 8. * drift) + 12. * (1. - scale));
            node.top = percent(25. + index.abs() * 8. - 14. * drift);
            let memory = memory_picture(crew.selected, part.0);
            image.image = if let Some(path) = memory { assets.load(path) }
                else { assets.load(format!("crew/dream/{}.png", NAMES[crew.selected.min(9)])) };
            let tint = match crew.appearances[crew.selected].coat {
                1 => (1.0, 0.96, 0.88), 2 => (0.78, 0.74, 0.70),
                3 => (0.68, 0.64, 0.62), 4 => (1.0, 0.95, 0.84),
                5 => (1.0, 0.88, 0.72), 6 => (0.82, 0.88, 0.95), _ => (1.,1.,1.),
            };
            // The three distinct Dachshund memories arrive in sequence and
            // continue drifting as they recede; all clear before the hand hold.
            let arrival = if memory.is_some() { (time - (part.0 - 1) as f32 * 0.35).clamp(0., 0.35) / 0.35 } else { 1. };
            image.color = Color::srgba(tint.0,tint.1,tint.2,picture_alpha(time) * arrival * darkness);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dream_only_runs_unconscious_and_fades_during_waking() {
        assert!(dream_time(conscious::Preview::Out.snapshot(1.), true).is_some());
        assert!(dream_time(conscious::Preview::Out.snapshot(1.), false).is_none());
        assert!(dream_time(conscious::State::default(), true).is_none());
        let wake = dream_time(conscious::Preview::Wake.snapshot(2.), true).unwrap();
        assert!(wake.1 > 0. && wake.1 < 1.);
        assert_eq!(picture_alpha(4.), 0.);
        assert!(finger_offset(1.0, 0) != finger_offset(1.0, 1));
        assert_eq!(memory_picture(7, 1), Some("crew/dream/dachshund-beach.png"));
        assert_eq!(memory_picture(7, 2), Some("crew/dream/dachshund-football.png"));
        assert_eq!(memory_picture(7, 3), Some("crew/dream/dachshund-wedding.png"));
        assert!(memory_picture(0, 1).is_none());
    }
}
