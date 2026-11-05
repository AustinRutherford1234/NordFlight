//! Window controls stay available while menus are open or flight is paused.
use bevy::{prelude::*, window::{MonitorSelection, PrimaryWindow, WindowMode}};

pub struct FullscreenPlugin;
impl Plugin for FullscreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, toggle_fullscreen);
    }
}

pub(crate) fn requested(keys:&ButtonInput<KeyCode>)->bool {
    // Windows may deliver a complete quick chord between two game frames.
    keys.just_pressed(KeyCode::Enter) && [KeyCode::AltLeft,KeyCode::AltRight].into_iter()
        .any(|alt|keys.pressed(alt)||keys.just_pressed(alt))
}
fn toggle_fullscreen(keys: Res<ButtonInput<KeyCode>>, mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut saved_size: Local<Option<bevy::window::WindowResolution>>) {
    if !requested(&keys) { return; }
    let Ok(mut window) = windows.single_mut() else { return; };
    if !window.focused { return; }
    if window.mode == WindowMode::Windowed {
        *saved_size=Some(window.resolution.clone());
        window.mode=WindowMode::BorderlessFullscreen(MonitorSelection::Current);
    } else {
        window.mode=WindowMode::Windowed;
        if let Some(size)=saved_size.take() {window.resolution=size;}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alt_enter_toggles_only_the_focused_primary_window() {
        let mut app=App::new();
        app.init_resource::<ButtonInput<KeyCode>>().add_plugins(FullscreenPlugin);
        let primary=app.world_mut().spawn((Window::default(),PrimaryWindow)).id();
        let secondary=app.world_mut().spawn(Window::default()).id();
        let size=app.world().get::<Window>(primary).unwrap().resolution.clone();
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Enter);
        app.update();
        assert_eq!(app.world().get::<Window>(primary).unwrap().mode,WindowMode::Windowed);
        for alt in [KeyCode::AltLeft,KeyCode::AltRight] {
            let mut keys=app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();keys.press(alt);keys.press(KeyCode::Enter);
            // Include quick chords whose modifier is released before Update.
            keys.release(alt);
            app.update();
            assert_eq!(app.world().get::<Window>(primary).unwrap().mode,WindowMode::BorderlessFullscreen(MonitorSelection::Current));
            app.world_mut().resource_mut::<ButtonInput<KeyCode>>().clear();
            app.update();
            assert_ne!(app.world().get::<Window>(primary).unwrap().mode,WindowMode::Windowed);
            app.world_mut().get_mut::<Window>(primary).unwrap().resolution.set_physical_resolution(2560,1440);
            app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::Enter);
            app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(alt);
            app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Enter);
            app.update();
            assert_eq!(app.world().get::<Window>(primary).unwrap().mode,WindowMode::Windowed);
            assert_eq!(app.world().get::<Window>(primary).unwrap().resolution,size);
        }
        app.world_mut().get_mut::<Window>(primary).unwrap().focused=false;
        let mut keys=app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();keys.press(KeyCode::AltLeft);keys.press(KeyCode::Enter);
        app.update();
        assert_eq!(app.world().get::<Window>(primary).unwrap().mode,WindowMode::Windowed);
        assert_eq!(app.world().get::<Window>(secondary).unwrap().mode,WindowMode::Windowed);
        assert_eq!(app.world().get::<Window>(primary).unwrap().resolution,size);
    }
}
