//! Static carrier import check. Run `cargo run --example nimitz_asset_preview`.
//! Set NORD_NIMITZ_CAPTURE=1 to save tests/nimitz-import.png and exit.
use bevy::{prelude::*,render::view::screenshot::Screenshot};
fn main() {
 App::new().add_plugins(DefaultPlugins.set(WindowPlugin{primary_window:Some(Window{title:"Nimitz carrier import preview".into(),resolution:(1400,900).into(),..default()}),..default()}))
 .insert_resource(ClearColor(Color::srgb(0.45,0.65,0.78)))
 .add_systems(Startup,setup).add_systems(Update,capture).run();
}
fn setup(mut c:Commands,assets:Res<AssetServer>,mut meshes:ResMut<Assets<Mesh>>,mut materials:ResMut<Assets<StandardMaterial>>) {
 // Preserve the downloaded GLB. Center its offset origin and normalize the
 // complete visual length to 333m; deck/collision dimensions remain unverified.
 let scale=333.0/303.842285_f32;
 c.spawn((SceneRoot(assets.load(GltfAssetLabel::Scene(0).from_asset("ships/nimitz/source.glb"))),Transform::from_xyz(-222.12742*scale,0.,-51.34455*scale).with_scale(Vec3::splat(scale))));
 c.spawn((Mesh3d(meshes.add(Plane3d::default().mesh().size(4000.,4000.))),MeshMaterial3d(materials.add(StandardMaterial{base_color:Color::srgb(0.025,0.15,0.20),perceptual_roughness:0.4,..default()})),Transform::from_xyz(0.,-0.15,0.)));
 c.spawn((Camera3d::default(),Transform::from_xyz(280.,195.,310.).looking_at(Vec3::new(0.,18.,0.),Vec3::Y),AmbientLight{color:Color::WHITE,brightness:450.,..default()}));
 c.spawn((DirectionalLight{illuminance:25000.,shadows_enabled:true,..default()},Transform::from_xyz(-200.,400.,100.).looking_at(Vec3::ZERO,Vec3::Y)));
}
fn capture(mut c:Commands,time:Res<Time>,meshes:Query<(),With<Mesh3d>>,mut done:Local<bool>) {
 if std::env::var_os("NORD_NIMITZ_CAPTURE").is_none() || *done || time.elapsed_secs()<15. || meshes.iter().count()<18 {return;}
 *done=true;
 println!("Carrier scene instantiated: {} mesh entities including ocean",meshes.iter().count());
 c.spawn(Screenshot::primary_window()).observe(|event:On<bevy::render::view::screenshot::ScreenshotCaptured>|{
  event.image.clone().try_into_dynamic().unwrap().to_rgb8().save("tests/nimitz-import.png").unwrap();
  std::process::exit(0);
 });
}
