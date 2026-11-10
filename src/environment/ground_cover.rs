//! Cosmetic ground cover placement in body-fixed coordinates. Never collidable.
use super::*;
pub const CELL:i32=64;
pub const GRID:i32=64;
#[derive(Clone,Debug,PartialEq)]
pub struct Tuft {pub position:DVec3,pub heading:f64,pub height:f32,pub width:f32,pub color:[f32;3],pub broadleaf:bool}
fn airport(x:f64,z:f64)->bool{
    (x.abs()<27.&&z.abs()<1260.)||((x-175.).abs()<140.&&(z-550.).abs()<510.)
}
fn suitability(cover:LandCover,water:bool,slope:f64)->f64{
    if water||slope>0.16{return 0.;}
    match cover{LandCover::Grass=>1.,LandCover::Shrub=>0.8,LandCover::Cropland=>0.7,
        LandCover::Wetland=>0.65,LandCover::Trees=>0.18,_=>0.}
}
pub fn placements(world:&EnvironmentSnapshot,cx:i32,cz:i32,density:f64)->Vec<Tuft>{
    if density<=0.||world.preset.vegetation.density<=0.{return vec![];}
    let seed=world.preset.seed.wrapping_add(0x6772617373);
    let mut out=Vec::with_capacity(256);
    // Large props share their authoritative deterministic placement service.
    let mut props=Vec::new();
    for z in (cz*CELL-16).div_euclid(128)..=(cz*CELL+CELL+16).div_euclid(128){
        for x in (cx*CELL-16).div_euclid(128)..=(cx*CELL+CELL+16).div_euclid(128){
            if let Some(p)=super::prop_in(world,x,z){props.push(p);}
        }
    }
    for z in 0..GRID{for x in 0..GRID{
        let ix=cx*GRID+x;let iz=cz*GRID+z;
        let px=(ix as f64+0.08+hash(ix,iz,seed)*0.84)*CELL as f64/GRID as f64;
        let pz=(iz as f64+0.08+hash(ix,iz,seed.wrapping_add(1))*0.84)*CELL as f64/GRID as f64;
        if airport(px,pz){continue;}
        let cluster=0.4+noise(px,pz,23.,seed.wrapping_add(2))*0.6;
        let chance=hash(ix,iz,seed.wrapping_add(3));
        if chance>density*cluster{continue;}
        let h=world.height_xz(px,pz);
        if h>world.preset.terrain.snow_line{continue;}
        let position=point(px,pz,h,world.preset.planet.radius);
        let sample=world.sample_surface(position);
        if chance>density*cluster*suitability(sample.land_cover,sample.water_surface.is_some(),1.-sample.normal.dot(position.normalize())){continue;}
        if props.iter().any(|p|(position.x-p.position.x).abs()<p.half.x+0.5&&(position.z-p.position.z).abs()<p.half.z+0.5){continue;}
        let broadleaf=matches!(sample.land_cover,LandCover::Shrub|LandCover::Trees|LandCover::Wetland)
            &&hash(ix,iz,seed.wrapping_add(4))<0.28;
        let scale=world.preset.vegetation.height_scale.clamp(0.5,2.)as f32;
        let height=(if broadleaf{0.5}else{0.22})+hash(ix,iz,seed.wrapping_add(5))as f32*if broadleaf{0.4}else{0.43};
        let shade=1.1+hash(ix,iz,seed.wrapping_add(6))as f32*0.45;
        let color=world.preset.palette[if sample.land_cover==LandCover::Cropland{1}else{0}]
            .map(|c|super::srgb_to_linear((c*shade).min(0.95)));
        let mown=if px.abs()<105.&&pz.abs()<1450.{0.4}else{1.};
        out.push(Tuft{position:position-position.normalize()*0.06,heading:hash(ix,iz,seed.wrapping_add(7))*std::f64::consts::TAU,
            height:height*scale*mown,width:(0.75+hash(ix,iz,seed.wrapping_add(8))as f32*0.5)*scale,color,broadleaf});
    }}
    out
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn surface_policy_excludes_water_pavement_bare_ground_and_cliffs(){
        for c in [LandCover::Water,LandCover::Built,LandCover::Rock,LandCover::Snow]{assert_eq!(suitability(c,false,0.),0.);}
        assert_eq!(suitability(LandCover::Grass,true,0.),0.);
        assert_eq!(suitability(LandCover::Grass,false,0.3),0.);
        assert!(airport(0.,0.)&&airport(175.,550.)&&!airport(1000.,1000.));
    }
    #[test]fn placement_is_stable_bounded_and_density_is_nested(){
        let world=EnvironmentSnapshot::legacy();
        let a=placements(&world,200,100,0.65);let b=placements(&world,200,100,0.65);
        assert_eq!(a,b);assert!(!a.is_empty());assert!(a.len()<=(GRID*GRID)as usize);
        let dense=placements(&world,200,100,1.);
        assert!(a.iter().all(|p|dense.contains(p)));
        assert!(a.iter().all(|p|p.position.is_finite()&&p.height>0.&&p.height<2.));
        assert!(placements(&world,0,0,1.).iter().all(|p|!airport(p.position.x,p.position.z)));
        assert!(placements(&world,200,100,0.).is_empty());
    }
}
