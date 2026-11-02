//! Shared imported Su-27P exterior for combat aircraft; simulation state is unchanged.
use super::*;
#[derive(Component)]
pub struct FighterModel {slot:usize,level:usize}
pub fn setup(mut c:Commands,server:Res<AssetServer>) {
    let scenes:[Handle<Scene>;3]=std::array::from_fn(|i|crate::model_assets::load(&server,GltfAssetLabel::Scene(0).from_asset(format!("aircraft/su27/lod{i}.glb"))));
    for slot in 0..MAX_AIRCRAFT {for (level,scene) in scenes.iter().enumerate(){
        c.spawn((FighterModel{slot,level},SceneRoot(scene.clone()),Transform::default(),Visibility::Hidden));
    }}
}
fn is_fighter(c:&Combat,slot:usize)->bool {
    c.active && c.targets.get(slot).is_some_and(|t|t.alive)
        && c.naval.as_ref().is_none_or(|n|n.raiders.iter().any(|r|r.slot==slot && r.role==naval::Role::Fighter))
}
fn lod(distance:f32,old:usize)->usize {
    match old {0 if distance<880.=>0,1 if (720. ..3300.).contains(&distance)=>1,2 if distance>2700.=>2,
        _=>if distance<800.{0}else if distance<3000.{1}else{2}}
}
pub fn update(f:Res<Flight>,c:Res<Combat>,camera:Query<&GlobalTransform,With<crate::FlightCamera>>,
    mut previous:Local<[usize;MAX_AIRCRAFT]>,mut roots:Query<(&FighterModel,&mut Transform,&mut Visibility)>) {
    let mut selected=[None;MAX_AIRCRAFT];
    for slot in 0..MAX_AIRCRAFT {if is_fighter(&c,slot) {
        let t=&c.targets[slot];let p=world::local(t.position,&f);
        let distance=camera.iter().next().map_or(f.position.distance(t.position)as f32,|cam|cam.translation().distance(p));
        let level=lod(distance,previous[slot]);previous[slot]=level;selected[slot]=Some(level);
    }}
    for (root,mut transform,mut visibility) in &mut roots {
        let shown=selected[root.slot]==Some(root.level);
        visibility.set_if_neq(if shown {Visibility::Inherited}else{Visibility::Hidden});
        if shown {let t=&c.targets[root.slot];transform.translation=world::local(t.position,&f);transform.rotation=world::local_rot(t.orientation,&f);}
    }
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn lod_has_close_detail_and_hysteresis(){assert_eq!(lod(50.,2),0);assert_eq!(lod(850.,0),0);assert_eq!(lod(1000.,0),1);assert_eq!(lod(3200.,1),1);assert_eq!(lod(5000.,1),2);}
    #[test]fn combat_fighters_are_included_and_bombers_excluded(){let f=naval::flight(false);let mut c=Combat::default();naval::start(&mut c,&f,1,1);assert!(is_fighter(&c,0));assert!(!is_fighter(&c,2));c.targets[0].alive=false;assert!(!is_fighter(&c,0));c.naval=None;c.targets[0].alive=true;assert!(is_fighter(&c,0));}
}
