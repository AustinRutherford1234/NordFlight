#[path="../src/aircraft/visual/hornet_gear.rs"]
mod gear_motion;
// Standalone import check for the licensed F/A-18F source model.
// Run with `cargo run --example fa18f_asset_preview`; G switches gear states,
// O toggles the canopy, H toggles the hook, and W folds the wings.
use bevy::{prelude::*, render::view::screenshot::Screenshot};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "F/A-18F asset preview".into(), resolution: (1200, 800).into(), ..default() }),
            ..default()
        }))
        .insert_resource(GearPreview { down: true, extension: 1. })
        .init_resource::<Mechanisms>()
        .add_systems(Startup, setup)
        .add_systems(Update, (capture, gear_controls, mechanism_controls).chain())
        .run();
}

#[derive(Resource)]
struct GearPreview { down: bool, extension: f32 }

#[derive(Resource, Default)]
struct Mechanisms {
    canopy_open: bool,
    hook_down: bool,
    wings_folded: bool,
    canopy_angle: f32,
    hook_angle: f32,
    wing_angle: f32,
    surfaces: bool,
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    if std::env::var_os("NORD_FA18F_OPEN").is_some() {
        commands.insert_resource(Mechanisms { canopy_open: true, hook_down: true, wings_folded: true, surfaces: true, ..default() });
    }
    if std::env::var_os("NORD_FA18F_SURFACES").is_some() && std::env::var_os("NORD_FA18F_OPEN").is_none() {
        commands.insert_resource(Mechanisms { surfaces: true, ..default() });
    }
    if std::env::var_os("NORD_FA18F_UP").is_some() {
        commands.insert_resource(GearPreview { down: false, extension: 0.45 });
    }
    if let Ok(value)=std::env::var("NORD_FA18F_GEAR_POSE") {
        if let Ok(value)=value.parse::<f32>() { commands.insert_resource(GearPreview { down: true, extension: value.clamp(0.,1.) }); }
    }
    commands.spawn((
        SceneRoot(assets.load(GltfAssetLabel::Scene(0).from_asset("aircraft/fa18f/rigged.glb"))),
        Transform::from_xyz(0., 0., 2.56)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::splat(0.1)),
    ));
    let camera = if std::env::var_os("NORD_FA18F_UNDERSIDE").is_some() { Vec3::new(15., -8., -17.) } else { Vec3::new(15., 10., -17.) };
    let (camera,focus)=match std::env::var("NORD_FA18F_CLOSEUP").as_deref() {
        Ok("nose")=>(Vec3::new(2.8,-2.8,-4.2),Vec3::new(0.,-0.7,-4.2)),
        Ok("main")=>(Vec3::new(3.8,-2.8,2.),Vec3::new(0.,-0.8,2.)),
        Ok("hook")=>(Vec3::new(4.,-3.5,9.7),Vec3::new(0.,-1.,6.8)),
        _=>(camera,Vec3::ZERO),
    };
    commands.spawn((Camera3d::default(), Transform::from_translation(camera).looking_at(focus, Vec3::Y), AmbientLight { color: Color::WHITE, brightness: 450., ..default() }));
    commands.spawn((DirectionalLight { illuminance: 25_000., ..default() }, Transform::from_xyz(8., 20., -12.).looking_at(Vec3::ZERO, Vec3::Y)));
}

fn mechanism_controls(keys: Res<ButtonInput<KeyCode>>, time: Res<Time>, mut state: ResMut<Mechanisms>, mut nodes: Query<(&Name, &mut Transform)>) {
    if keys.just_pressed(KeyCode::KeyO) { state.canopy_open = !state.canopy_open; }
    if keys.just_pressed(KeyCode::KeyH) { state.hook_down = !state.hook_down; }
    if keys.just_pressed(KeyCode::KeyW) { state.wings_folded = !state.wings_folded; }
    if keys.just_pressed(KeyCode::KeyE) { state.surfaces = !state.surfaces; }
    let step = time.delta_secs().min(0.1);
    let canopy_target = if state.canopy_open { 0.48 } else { 0. };
    let hook_target = if state.hook_down || std::env::var_os("NORD_FA18F_HOOK_DOWN").is_some() { 0.65 } else { 0. };
    let wing_target = if state.wings_folded { 1.3 } else { 0. };
    state.canopy_angle += (canopy_target - state.canopy_angle).clamp(-0.15*step, 0.15*step);
    state.hook_angle += (hook_target - state.hook_angle).clamp(-0.32*step, 0.32*step);
    state.wing_angle += (wing_target - state.wing_angle).clamp(-0.26*step, 0.26*step);
    for (name, mut transform) in &mut nodes {
        let surface = if state.surfaces { 1.0 } else { 0.0 };
        let rotation = match name.as_str() {
            "HornetCanopyPivot" | "HornetCanopyFrame" => Quat::from_rotation_z(state.canopy_angle),
            "HornetHookPivot" => Quat::from_rotation_z(state.hook_angle),
            "HornetWingTipPivotL" => Quat::from_rotation_x(-state.wing_angle),
            "HornetWingTipPivotR" => Quat::from_rotation_x(state.wing_angle),
            "HornetPanel0" | "HornetPanel4" => Quat::from_rotation_z(0.22*surface),
            "HornetPanel1" | "HornetPanel5" => Quat::from_rotation_z(0.4*surface),
            "HornetPanel2" => Quat::from_rotation_z(-0.25*surface),
            "HornetPanel6" => Quat::from_rotation_z(0.25*surface),
            "HornetPanel3" | "HornetPanel7" => Quat::from_rotation_y(0.2*surface),
            "HornetPanel9" | "HornetPanel10" => Quat::from_rotation_z(-0.18*surface),
            _ => continue,
        };
        transform.rotation = rotation;
    }
}

fn gear_controls(keys: Res<ButtonInput<KeyCode>>, time: Res<Time>, mut gear: ResMut<GearPreview>, mut nodes: Query<(Entity, &Name, &mut Transform, &mut Visibility)>) {
    if keys.just_pressed(KeyCode::KeyG) { gear.down = !gear.down; }
    let target = if gear.down { 1. } else { 0. };
    let step = if std::env::var_os("NORD_FA18F_GEAR_POSE").is_some() {0.} else {time.delta_secs().min(0.1) * 0.30};
    gear.extension += (target - gear.extension).clamp(-step, step);
    let retract = 1. - gear.extension;
    for (_entity, name, mut transform, mut visibility) in &mut nodes {
        let (visible, rotation) = if name.as_str().starts_with("F18-landingOff") { (gear.extension < 0.01, None) }
            else if name.as_str().starts_with("F18-landingOnLight") { (gear.extension > 0.99, None) }
            else if name.as_str() == "HornetNoseBrace" { (true, Some(Quat::from_rotation_z(retract*0.55))) }
            else if name.as_str() == "HornetGearNose" { (true, Some(Quat::from_rotation_z(retract * 1.35))) }
            else if name.as_str() == "HornetGearMainL" { (true, Some(Quat::from_rotation_z(-retract * 0.35) * Quat::from_rotation_x(-retract * 1.35))) }
            else if name.as_str() == "HornetGearMainR" { (true, Some(Quat::from_rotation_z(-retract * 0.35) * Quat::from_rotation_x(retract * 1.35))) }
            else if name.as_str() == "HornetMainDoorL" { (true, Some(Quat::from_rotation_x(-1.2 * if gear.extension > 0.01 && gear.extension < 0.99 {1.} else {0.}))) }
            else if name.as_str() == "HornetMainDoorR" { (true, Some(Quat::from_rotation_x(1.2 * if gear.extension > 0.01 && gear.extension < 0.99 {1.} else {0.}))) }
            else if name.as_str() == "HornetNoseDoorL" { (true, Some(Quat::from_rotation_x(-1.2 * if gear.extension > 0.01 && gear.extension < 0.99 {1.} else {0.}))) }
            else if name.as_str() == "HornetNoseDoorR" { (true, Some(Quat::from_rotation_x(1.2 * if gear.extension > 0.01 && gear.extension < 0.99 {1.} else {0.}))) }
            else if name.as_str().starts_with("HornetStrut") || name.as_str().starts_with("HornetWheel") || name.as_str().starts_with("HornetMain") || name.as_str().starts_with("HornetNose") { (true,None) }
            else { continue; };
        if let Some(rotation) = rotation { transform.rotation = rotation; }
        if let Some(pose)=gear_motion::pose(name.as_str(),gear.extension,std::env::var("NORD_FA18F_DOOR_POSE").ok().and_then(|s|s.parse().ok()).unwrap_or(if gear.extension>0. && gear.extension<1. {1.}else{0.})) {
            if name.as_str().starts_with("HornetStrut") || name.as_str().starts_with("HornetWheel") { *transform=pose; }
            else { transform.rotation=pose.rotation; }
        }
        visibility.set_if_neq(if visible { Visibility::Inherited } else { Visibility::Hidden });
    }
}

fn capture(mut commands: Commands, time: Res<Time>, mut captured: Local<bool>, mut gear: ResMut<GearPreview>, mut index: Local<usize>, mut since: Local<f32>) {
    if std::env::var_os("NORD_FA18F_SEQUENCE").is_some() {
        let states=[(0.,0.),(0.,0.5),(0.,1.),(0.25,1.),(0.5,1.),(0.75,1.),(1.,1.),(1.,0.5),(1.,0.),(1.,0.5),(1.,1.),(0.75,1.),(0.5,1.),(0.25,1.),(0.,1.),(0.,0.5),(0.,0.)];
        if *index>=states.len() { std::process::exit(0); }
        let (extension,door)=states[*index];
        gear.extension=extension;
        unsafe {std::env::set_var("NORD_FA18F_DOOR_POSE",door.to_string());}
        let elapsed=time.elapsed_secs()-*since;
        if elapsed>=4. { *index+=1; *since=time.elapsed_secs(); *captured=false; return; }
        if elapsed<2. || *captured {return;}
        *captured=true;
        let path=format!("tests/fa18f-gear-sequence/sequence-{:02}.png",*index);
        commands.spawn(Screenshot::primary_window()).observe(move |event: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
            event.image.clone().try_into_dynamic().unwrap().to_rgb8().save(&path).unwrap();
        });
        return;
    }
    if std::env::var_os("NORD_FA18F_CAPTURE").is_none() || *captured || time.elapsed_secs() < 8. { return; }
    *captured = true;
    commands.spawn(Screenshot::primary_window()).observe(|event: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
        event.image.clone().try_into_dynamic().unwrap().to_rgb8().save("/tmp/nord-fa18f-bevy.png").unwrap();
        std::process::exit(0);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gear_toggle_selects_only_the_requested_assembly() {
        use std::time::Duration;
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>().init_resource::<Time>().insert_resource(GearPreview { down:true, extension:1. }).add_systems(Update, gear_controls);
        let up = app.world_mut().spawn((Name::new("F18-landingOff_4"), Transform::default(), Visibility::Inherited)).id();
        let down = app.world_mut().spawn((Name::new("HornetGearNose"), Transform::default(), Visibility::Inherited)).id();
        let light = app.world_mut().spawn((Name::new("F18-landingOnLight_6"), Transform::default(), Visibility::Inherited)).id();
        app.update();
        assert_eq!(*app.world().get::<Visibility>(up).unwrap(), Visibility::Hidden);
        assert_eq!(*app.world().get::<Visibility>(down).unwrap(), Visibility::Inherited);
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyG);
        app.world_mut().resource_mut::<Time>().advance_by(Duration::from_millis(100));
        app.update();
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
        assert_eq!(*app.world().get::<Visibility>(down).unwrap(), Visibility::Inherited);
        assert_eq!(app.world().get::<Transform>(down).unwrap().rotation, Quat::IDENTITY);
        for _ in 0..40 {
            app.world_mut().resource_mut::<Time>().advance_by(Duration::from_millis(100));
            app.update();
        }
        assert_eq!(*app.world().get::<Visibility>(up).unwrap(), Visibility::Inherited);
        assert_eq!(*app.world().get::<Visibility>(down).unwrap(), Visibility::Inherited);
        assert_eq!(*app.world().get::<Visibility>(light).unwrap(), Visibility::Hidden);
    }

    #[test]
    fn canopy_and_hook_follow_independent_commands_and_return_to_stow() {
        use std::time::Duration;
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>().init_resource::<Time>()
            .init_resource::<Mechanisms>().add_systems(Update, mechanism_controls);
        let canopy = app.world_mut().spawn((Name::new("HornetCanopyPivot"), Transform::from_xyz(40.39, 6.05, 0.))).id();
        let hook = app.world_mut().spawn((Name::new("HornetHookPivot"), Transform::from_xyz(-38., -7., 0.))).id();
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyO);
        app.world_mut().resource_mut::<Time>().advance_by(Duration::from_millis(100));
        app.update();
        assert!(app.world().resource::<Mechanisms>().canopy_angle > 0.);
        assert_eq!(app.world().resource::<Mechanisms>().hook_angle, 0.);
        assert_eq!(app.world().get::<Transform>(canopy).unwrap().translation, Vec3::new(40.39, 6.05, 0.));
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::KeyH);
        }
        app.world_mut().resource_mut::<Time>().advance_by(Duration::from_millis(100));
        app.update();
        assert!(app.world().resource::<Mechanisms>().hook_angle > 0.);
        assert_eq!(app.world().get::<Transform>(hook).unwrap().translation, Vec3::new(-38., -7., 0.));
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::KeyO);
            keys.press(KeyCode::KeyH);
        }
        app.world_mut().resource_mut::<Time>().advance_by(Duration::from_millis(100));
        app.update();
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
        for _ in 0..10 {
            app.world_mut().resource_mut::<Time>().advance_by(Duration::from_millis(100));
            app.update();
        }
        assert_eq!(app.world().resource::<Mechanisms>().canopy_angle, 0.);
        assert_eq!(app.world().resource::<Mechanisms>().hook_angle, 0.);
    }

    #[test]
    fn wing_fold_rotates_both_tips_up_and_restores_them() {
        use std::time::Duration;
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>().init_resource::<Time>()
            .init_resource::<Mechanisms>().add_systems(Update, mechanism_controls);
        let l = app.world_mut().spawn((Name::new("HornetWingTipPivotL"), Transform::default())).id();
        let r = app.world_mut().spawn((Name::new("HornetWingTipPivotR"), Transform::default())).id();
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyW);
        app.world_mut().resource_mut::<Time>().advance_by(Duration::from_millis(100));
        app.update();
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
        for _ in 0..55 {
            app.world_mut().resource_mut::<Time>().advance_by(Duration::from_millis(100));
            app.update();
        }
        let left = app.world().get::<Transform>(l).unwrap().rotation;
        let right = app.world().get::<Transform>(r).unwrap().rotation;
        assert!((left * Vec3::Z).y > 0.);
        assert!((right * -Vec3::Z).y > 0.);
        assert!((app.world().resource::<Mechanisms>().wing_angle - 1.3).abs() < 1e-5);
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyW);
        app.world_mut().resource_mut::<Time>().advance_by(Duration::from_millis(100));
        app.update();
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
        for _ in 0..55 {
            app.world_mut().resource_mut::<Time>().advance_by(Duration::from_millis(100));
            app.update();
        }
        assert_eq!(app.world().resource::<Mechanisms>().wing_angle, 0.);
    }
}
