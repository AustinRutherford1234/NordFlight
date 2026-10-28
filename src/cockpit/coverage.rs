//! Crop only zero-contribution HUD borders; retain the original UV/plane mapping.
use bevy::{prelude::*,math::Affine2};
#[derive(Component)]
pub(crate) struct Panel {
    pub rest:Transform,
    pub size:Vec2,
    pub empty:bool,
}
fn bounds(data:&[u8],width:usize,height:usize)->Option<[usize;4]> {
    assert_eq!(data.len(),width*height*4);
    let (mut left,mut top,mut right,mut bottom)=(width,height,0,0);
    for (i,pixel) in data.chunks_exact(4).enumerate() {
        if pixel[3]!=0 {let x=i%width;let y=i/width;left=left.min(x);top=top.min(y);right=right.max(x+1);bottom=bottom.max(y+1);}
    }
    (right>left && bottom>top).then(||[left.saturating_sub(2),top.saturating_sub(2),(right+2).min(width),(bottom+2).min(height)])
}
pub(super) fn apply(panel:&mut Panel,transform:&mut Transform,visibility:&mut Visibility,
    materials:&mut Assets<StandardMaterial>,handle:&Handle<StandardMaterial>,data:&[u8]) {
    let width=if data.len()==1024*768*4 {1024}else{256};
    let height=width*3/4;
    let Some([left,top,right,bottom])=bounds(data,width,height) else {
        panel.empty=true;*visibility=Visibility::Hidden;return;
    };
    panel.empty=false;*visibility=Visibility::Inherited;
    let min=Vec2::new(left as f32/width as f32,top as f32/height as f32);
    let max=Vec2::new(right as f32/width as f32,bottom as f32/height as f32);
    let span=max-min;let centre=(min+max)*0.5;
    let crop=Transform::from_translation(Vec3::new((centre.x-0.5)*panel.size.x,(0.5-centre.y)*panel.size.y,0.))
        .with_scale(Vec3::new(span.x,span.y,1.));
    let desired=panel.rest*crop;if *transform!=desired {*transform=desired;}
    let uv=Affine2::from_scale_angle_translation(span,0.,min);
    if materials.get(handle).is_some_and(|m|m.uv_transform!=uv) {materials.get_mut(handle).unwrap().uv_transform=uv;}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_every_nonzero_alpha_texel_with_filter_support() {
        let mut data=vec![0;256*192*4];
        data[(70*256+33)*4+3]=1;data[(99*256+180)*4+3]=255;
        assert_eq!(bounds(&data,256,192),Some([31,68,183,102]));
        data[3]=1;data[(192*256-1)*4+3]=1;
        assert_eq!(bounds(&data,256,192),Some([0,0,256,192]));
    }
    #[test]
    fn high_resolution_hud_retains_uv_alignment() {
        let mut panel=Panel{rest:Transform::IDENTITY,size:Vec2::new(0.3,0.18),empty:true};let mut transform=panel.rest;let mut visibility=Visibility::Hidden;
        let mut materials=Assets::<StandardMaterial>::default();let handle=materials.add(StandardMaterial::default());let mut data=vec![0;1024*768*4];
        data[(280*1024+132)*4+3]=255;
        apply(&mut panel,&mut transform,&mut visibility,&mut materials,&handle,&data);assert!(!panel.empty);
        let mapped=materials.get(&handle).unwrap().uv_transform.transform_point2(Vec2::splat(0.5));
        assert!((mapped.x-132.5/1024.).abs()<1e-6);assert!((mapped.y-280.5/768.).abs()<1e-6);
    }
    #[test]
    fn cropped_plane_uvs_match_full_plane_and_empty_recovers() {
        let mut panel=Panel {rest:Transform::from_xyz(1.,2.,-3.).with_rotation(Quat::from_rotation_x(-0.14)),size:Vec2::new(0.3,0.18),empty:false};
        let mut transform=panel.rest;let mut visibility=Visibility::Inherited;
        let mut materials=Assets::<StandardMaterial>::default();let handle=materials.add(StandardMaterial::default());
        let mut data=vec![0;256*192*4];
        apply(&mut panel,&mut transform,&mut visibility,&mut materials,&handle,&data);
        assert!(panel.empty);assert_eq!(visibility,Visibility::Hidden);
        data[(70*256+33)*4+3]=1;
        apply(&mut panel,&mut transform,&mut visibility,&mut materials,&handle,&data);
        assert!(!panel.empty);assert_eq!(visibility,Visibility::Inherited);
        for uv in [Vec2::ZERO,Vec2::ONE,Vec2::splat(0.37)] {
            let mapped=materials.get(&handle).unwrap().uv_transform.transform_point2(uv);
            let original=panel.rest.transform_point(Vec3::new((mapped.x-0.5)*panel.size.x,(0.5-mapped.y)*panel.size.y,0.));
            let cropped=transform.transform_point(Vec3::new((uv.x-0.5)*panel.size.x,(0.5-uv.y)*panel.size.y,0.));
            assert!(original.distance(cropped)<1e-6);
        }
        assert!(transform.scale.min_element()>0.);
    }
}
