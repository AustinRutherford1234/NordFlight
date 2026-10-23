//! Shared main-bay presentation and release geometry. Publicly illustrated
//! bifold layout; hidden linkage dimensions and timing are game approximations.
use super::*;
pub const HALF:f32=0.95;
pub const FORE:f32=-1.85;
pub const AFT:f32=2.55;
pub const ROOF:f32=-0.28;
pub const STROKE:f64=0.2286;
pub fn edge(x:f32,aft:bool)->f32 {
    let u=(x.abs()/HALF*8.).clamp(0.,8.);
    let tooth=if (u.floor()as usize)%2==0{u.fract()}else{1.-u.fract()};
    if aft{AFT-0.055*tooth}else{FORE+0.055*tooth}
}
pub fn pivot(side:f32,inner:bool)->Vec3 {
    Vec3::new(side*if inner{HALF*0.5}else{HALF+0.02},-1.04,0.)
}
pub fn door_pose(side:f32,inner:bool,progress:f64)->Transform {
    let p=progress as f32;
    let angle=bays::smooth(p)*95_f32.to_radians();
    let outer=Transform::from_translation(pivot(side,false)).with_rotation(Quat::from_rotation_z(side*angle));
    if !inner{return outer;}
    let fold=bays::smooth((p-0.18)/0.82)*140_f32.to_radians();
    Transform::from_translation(outer.transform_point(pivot(side,true)-pivot(side,false)))
        .with_rotation(outer.rotation*Quat::from_rotation_z(side*fold))
}
pub fn station(slot:usize)->DVec3 {
    let slot=slot.min(5);
    DVec3::new((slot as f64-2.5)*0.31,-0.5,if slot==1||slot==4{0.15}else{0.525})
}
pub fn store_position(slot:usize,stroke:f64)->DVec3 {
    station(slot)-DVec3::Y*(STROKE*stroke.clamp(0.,1.))
}
pub fn ready(s:&Systems,slot:usize)->bool {s.bays>=0.999&&s.main_stroke[slot.min(5)]>=0.999}
pub fn advance(s:&mut Systems,powered:bool,dt:f64) {
    if !powered||dt<=0.{return;}
    let open=s.bays_target||s.manual_bays||s.main_request.is_some();
    let available=if open {
        let used=dt.min((1.-s.bays)/2.5);
        s.bays=(s.bays+used*2.5).min(1.);dt-used
    }else{
        // Retract the longest outstanding stroke before spending any of the
        // step on door closing. Large timesteps cannot cross the mechanism.
        let used=s.main_stroke.iter().copied().fold(0_f64,f64::max)*0.18;
        s.bays=(s.bays-(dt-used).max(0.)*2.5).max(0.);dt
    };
    for i in 0..6 {
        let extend=s.main_request==Some(i)&&s.bays>=0.999;
        let target=if extend{1.}else{0.};
        s.main_stroke[i]+=(target-s.main_stroke[i]).clamp(-dt/0.18,available/0.10);
    }
}
pub fn suspension(slot:usize,stroke:f64,z:f32)->[Vec3;3] {
    let at=station(slot).as_vec3();
    let a=Vec3::new(at.x,ROOF-0.05,at.z+z);
    let b=store_position(slot,stroke).as_vec3()+Vec3::new(0.,0.10,z);
    let h=a.distance(b)*0.5;
    let offset=(0.17_f32.powi(2)-h*h).max(0.).sqrt();
    [a,(a+b)*0.5+Vec3::Z*offset*z.signum(),b]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_bifold_poses_preserve_authored_skin_and_open_outward() {
        for side in [-1.,1.] {for inner in [false,true] {
            assert_eq!(door_pose(side,inner,0.),Transform::from_translation(pivot(side,inner)));
            for k in 0..=100 {let t=door_pose(side,inner,k as f64/100.);assert!(t.translation.is_finite()&&t.rotation.is_finite());}
        }}
        let p=door_pose(1.,true,1.).transform_point(Vec3::new(-HALF*0.5,-0.13,0.));
        assert!(p.x>1.1&&p.y< -1.,"folded leaf must remain below/outside the bay: {p:?}");
    }
    #[test]
    fn ordered_stages_are_independent_of_step_size() {
        for dt in [0.001_f64,1./120.,0.1,1.] {
            let mut s=Systems::default();s.main_request=Some(0);
            let mut time=0.;while time<1. {let h=dt.min(1.-time);advance(&mut s,true,h);time+=h;}
            assert!(ready(&s,0));
            s.main_request=None;time=0.;
            while time<1. {let h=dt.min(1.-time);advance(&mut s,true,h);time+=h;}
            assert_eq!(s.main_stroke,[0.;6]);assert!(s.bays<1e-9);
        }
    }
    #[test]
    fn ejector_waits_for_doors_and_uses_fixed_length_suspension() {
        let mut s=Systems::default();s.main_request=Some(2);
        for _ in 0..30 {advance(&mut s,true,0.01);assert_eq!(s.main_stroke,[0.;6]);}
        for _ in 0..30 {advance(&mut s,true,0.01);}
        assert!(ready(&s,2));assert_eq!(s.main_stroke[1],0.);
        let frozen=s.clone();advance(&mut s,false,1.);assert_eq!(s.bays,frozen.bays);assert_eq!(s.main_stroke,frozen.main_stroke);
        for i in 0..6 {for k in 0..=100 {for z in [-0.65,0.65] {
            let p=suspension(i,k as f64/100.,z);
            assert!((p[0].distance(p[1])-0.17).abs()<1e-5);assert!((p[1].distance(p[2])-0.17).abs()<1e-5);
        }}}
        s.main_request=None;for _ in 0..100{advance(&mut s,true,0.01);}assert!(s.bays<1e-9);assert_eq!(s.main_stroke,[0.;6]);
    }
}
