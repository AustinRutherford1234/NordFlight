//! Photo-guided intake presentation. Internal routing is an approximation;
//! continuous S-bends conceal the engine face without an inlet blanking plate.
use super::*;

pub(super) const SLICES: usize = 64;
pub(super) fn lip(side: f32) -> [Vec3; 4] {
    [Vec3::new(side*1.02,0.20,-4.30), Vec3::new(side*2.07,0.18,-3.63),
     Vec3::new(side*2.12,-0.40,-3.48), Vec3::new(side*1.11,-0.44,-4.17)]
}
pub(super) fn ring(side: f32, u: f32) -> [Vec3; 4] {
    // u, centre x/y/z, half width/height. The first bend climbs and turns
    // inboard above the weapon bays; the return bend meets the engine casing.
    const ROUTE: [[f32;6];5] = [
        [0.,1.58,-0.21,-3.90,0.53,0.41],
        [0.18,1.20,-0.02,-2.40,0.34,0.14],
        [0.38,0.94,0.36,-0.60,0.36,0.13],
        [0.68,0.88,0.10,2.15,0.40,0.28],
        [1.,0.93,-0.12,5.02,0.52,0.40],
    ];
    let u=u.clamp(0.,1.);
    let i=ROUTE.windows(2).position(|p| u<=p[1][0]).unwrap();
    let t=(u-ROUTE[i][0])/(ROUTE[i+1][0]-ROUTE[i][0]);
    let value=|k:usize| {
        let tangent=|j:usize| {
            if j==0 {(ROUTE[1][k]-ROUTE[0][k])/(ROUTE[1][0]-ROUTE[0][0])}
            else if j==4 {(ROUTE[4][k]-ROUTE[3][k])/(ROUTE[4][0]-ROUTE[3][0])}
            else {(ROUTE[j+1][k]-ROUTE[j-1][k])/(ROUTE[j+1][0]-ROUTE[j-1][0])}
        };
        let h=ROUTE[i+1][0]-ROUTE[i][0];
        (2.*t*t*t-3.*t*t+1.)*ROUTE[i][k]+(t*t*t-2.*t*t+t)*h*tangent(i)
            +(-2.*t*t*t+3.*t*t)*ROUTE[i+1][k]+(t*t*t-t*t)*h*tangent(i+1)
    };
    let x=value(1);let y=value(2);let z=value(3);let w=value(4);let h=value(5);
    let first=lip(1.);
    let base=[Vec3::new(1.05,0.20,-3.90),Vec3::new(2.11,0.20,-3.90),
        Vec3::new(2.11,-0.62,-3.90),Vec3::new(1.05,-0.62,-3.90)];
    let fade=(1.-u/0.18).max(0.).powi(2);
    std::array::from_fn(|k| {
        let dx=if k==0||k==3{-w}else{w};let dy=if k<2{h}else{-h};
        let mut p=Vec3::new(x+dx,y+dy,z)+(first[k]-base[k])*fade;
        p.x*=side;p
    })
}
fn outer_ring(side: f32, u: f32) -> [Vec3; 4] {
    let inner=ring(side,u);
    // Substantial gray inlet surround, recessed mouth and a cowl that merges
    // into the body. The first pass exposed the black trunk as exterior skin.
    let blend=(1.-u/0.28).clamp(0.,1.);
    let width=0.12*blend*blend;
    std::array::from_fn(|k|inner[k]+Vec3::new(
        side*if k==0||k==3{-width}else{width},
        if k<2{width}else{-width},-0.085*blend*blend))
}
// The six tetrahedra and the four side quads use the same face diagonals.
// A single plane per twisted quad used to leave slivers of fuselage in the lip.
const TETS: [[usize;4];6]=[[0,1,2,6],[0,2,3,6],[0,3,7,6],[0,7,4,6],[0,4,5,6],[0,5,1,6]];
fn vertices(side:f32,j:usize,outer:bool)->[Vec3;8] {
    let at=|u|if outer{outer_ring(side,u)}else{ring(side,u)};
    let a=at(j as f32/SLICES as f32);let b=at((j+1)as f32/SLICES as f32);
    [a[0],a[1],a[2],a[3],b[0],b[1],b[2],b[3]]
}
fn walls(side: f32, outer: bool) -> Mesh {
    let mut v=Vec::new();let mut ix=Vec::new();
    for j in 0..SLICES {
        let p=vertices(side,j,outer);
        for face in [[0,4,5,1],[1,5,6,2],[3,2,6,7],[0,3,7,4]] {
            let mut q=face.map(|k|p[k]);
            if (side>0.)!=outer {q.reverse();}
            quad(&mut v,&mut ix,q);
        }
    }
    raptor_mesh::smooth(mesh(v,ix))
}
pub(super) fn trunk(side:f32)->Mesh {walls(side,false)}
fn surround(side:f32)->Mesh {
    let mut v=Vec::new();let mut ix=Vec::new();
    let outer=outer_ring(side,0.);let inner=ring(side,0.);
    for k in 0..4 {
        let n=(k+1)%4;
        let mut q=[outer[k],outer[n],inner[n],inner[k]];
        if side<0. {q.reverse();}
        quad(&mut v,&mut ix,q);
    }
    mesh(v,ix)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gray_surround_recesses_the_mouth_and_uses_the_same_cutting_diagonals() {
        use std::collections::BTreeMap;
        let mut faces=BTreeMap::new();
        for tet in TETS {
            for f in [[0,1,2],[0,3,1],[0,2,3],[1,3,2]] {
                let mut key=f.map(|i|tet[i]);key.sort();
                *faces.entry(key).or_insert(0)+=1;
            }
        }
        let mut expected=Vec::new();
        for face in [[0,4,5,1],[1,5,6,2],[3,2,6,7],[0,3,7,4],[0,1,2,3],[4,5,6,7]] {
            for mut triangle in [[face[0],face[1],face[2]],[face[0],face[2],face[3]]] {
                triangle.sort();expected.push(triangle);
            }
        }
        expected.sort();
        assert_eq!(faces.into_iter().filter_map(|(f,n)|(n==1).then_some(f)).collect::<Vec<_>>(),expected);
        for side in [-1.,1.] {
            for (outer,inner) in outer_ring(side,0.).into_iter().zip(ring(side,0.)) {
                assert!(outer.z<inner.z-0.08);
                assert!((outer.y-inner.y).abs()>0.11);
            }
            let surround=surround(side);
            assert_eq!(surround.count_vertices(),16);
            for normal in surround.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().as_float3().unwrap() {
                assert!(normal[2]< -0.1,"surround must face forward");
            }
        }
        assert!(cut_cells().iter().all(|c|c.planes.iter().all(|(n,d)|n.is_finite()&&d.is_finite())));
    }
    #[test]
    fn cowl_cutouts_meet_the_rendered_triangles_without_body_slivers() {
        let area=|pieces:Vec<Vec<Vec3>>|pieces.iter().map(|p|(1..p.len()-1)
            .map(|i|(p[i]-p[0]).cross(p[i+1]-p[0]).length()*0.5).sum::<f32>()).sum::<f32>();
        for side in [-1.,1.] {for j in 0..8 {
            let p=vertices(side,j,true);
            for face in [[0,4,5,1],[1,5,6,2],[3,2,6,7],[0,3,7,4]] {
                for t in [[face[0],face[1],face[2]],[face[0],face[2],face[3]]] {
                    let [a,b,c]=t.map(|k|p[k]);
                    let centre=(a+b+c)/3.;let n=(b-a).cross(c-a).normalize()*side;
                    let u=(b-a).normalize()*0.0001;let v=n.cross(u);
                    let sample=|sign|vec![centre+n*sign*0.001-u-v,centre+n*sign*0.001+u-v,centre+n*sign*0.001+v];
                    let outside=sample(1.);let expected=area(vec![outside.clone()]);
                    assert!(area(cut_openings(vec![sample(-1.)]))<expected*0.05,"body remains inside cowl at {j}");
                    assert!(area(cut_openings(vec![outside]))>expected*0.95,"gap outside cowl at {j}");
                }
            }
        }}
    }
    #[test]
    fn ducts_have_open_lips_and_continuous_walls_to_each_engine() {
        for side in [-1.,1.] {
            for (a,b) in ring(side,0.).into_iter().zip(lip(side)) {assert!(a.distance(b)<1e-6);}
            let end=ring(side,1.);
            let centre=end.iter().copied().sum::<Vec3>()/4.;
            assert!(centre.distance(Vec3::new(side*0.93,-0.12,5.02))<1e-5);
            let mesh=trunk(side);
            let positions=mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap();
            assert_eq!(positions.len(),SLICES*4*4);
            for p in positions {assert!(Vec3::from_array(*p).is_finite());}
            // No face spans an aperture: every quad is one wall of a slice.
            for q in positions.chunks_exact(4) {
                let shortest=(0..4).map(|k|Vec3::from_array(q[k]).distance(Vec3::from_array(q[(k+1)%4])))
                    .fold(f32::INFINITY,f32::min);
                assert!(shortest<0.35);
            }
            // Routing stays above/inboard of the side-bay and main-bay cavities.
            for j in 0..=256 {
                for p in ring(side,j as f32/256.) {
                    if p.z>=-1.95 && p.z<=0.75 {assert!(p.x.abs()<1.49||p.y> -0.10,"side bay: {p:?}");}
                    if p.z>=-1.75 && p.z<=1.75 {assert!(p.y> -0.28,"main bay: {p:?}");}
                }
            }
        }
    }
    #[test]
    fn skin_is_removed_from_the_actual_stream_tube() {
        let r=ring(1.,0.1);
        let centre=r.iter().copied().sum::<Vec3>()/4.;
        let plate=vec![centre+Vec3::new(-0.04,-0.04,0.),centre+Vec3::new(0.04,-0.04,0.),
            centre+Vec3::new(0.04,0.04,0.),centre+Vec3::new(-0.04,0.04,0.)];
        assert!(cut_openings(vec![plate]).is_empty());
        let outside=vec![Vec3::new(3.,0.,-3.),Vec3::new(3.1,0.,-3.),Vec3::new(3.,0.1,-3.)];
        assert_eq!(cut_openings(vec![outside.clone()]),vec![outside]);
    }
    #[test]
    fn s_bend_obscures_the_entire_engine_face_from_the_lip() {
        let r=lip(1.);let rear=ring(1.,1.);
        let mesh=trunk(1.);
        let p=mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap();
        let indices=mesh.indices().unwrap();
        let triangles=indices.iter().collect::<Vec<_>>();
        let sample=|q:[Vec3;4],s:f32,t:f32| q[0].lerp(q[1],s).lerp(q[3].lerp(q[2],s),t);
        for x in 0..=6 {for y in 0..=6 {for ex in 0..=4 {for ey in 0..=4 {
            let a=sample(r,0.02+0.96*x as f32/6.,0.02+0.96*y as f32/6.);
            let b=sample(rear,0.02+0.96*ex as f32/4.,0.02+0.96*ey as f32/4.);
            let dir=b-a;
            let blocked=triangles.chunks_exact(3).any(|i| {
                let v0=Vec3::from_array(p[i[0]]);let e1=Vec3::from_array(p[i[1]])-v0;let e2=Vec3::from_array(p[i[2]])-v0;
                let h=dir.cross(e2);let det=e1.dot(h);if det.abs()<1e-7 {return false;}
                let s=a-v0;let u=s.dot(h)/det;if !(0. ..=1.).contains(&u) {return false;}
                let q=s.cross(e1);let v=dir.dot(q)/det;let t=e2.dot(q)/det;
                v>=0.&&u+v<=1.&&t>0.0001&&t<0.9999
            });
            assert!(blocked,"direct engine sightline from {a:?} to {b:?}");
        }}}}
    }
}

// Subtract each short convex stream tube from the fuselage. Unlike bounding
// boxes this seats the swept lips exactly and leaves no solid skin in the duct.
struct CutCell {min:Vec3,max:Vec3,planes:[(Vec3,f32);4]}
fn cut_cells()-> &'static [CutCell] {
    static CELLS:std::sync::OnceLock<Vec<CutCell>>=std::sync::OnceLock::new();
    CELLS.get_or_init(|| {
        let mut cells=Vec::new();
        for side in [-1.,1.] {for j in 0..SLICES {
            let p=vertices(side,j,true);
            for tet in TETS {
                let p=tet.map(|k|p[k]);let centre=p.iter().copied().sum::<Vec3>()/4.;
                let planes=[[0,1,2],[0,3,1],[0,2,3],[1,3,2]].map(|face| {
                    let [a,b,c]=face.map(|k|p[k]);
                    let mut n=(b-a).cross(c-a).normalize();
                    if n.dot(centre-a)<0. {n=-n;}
                    (n,n.dot(a))
                });
                cells.push(CutCell {min:p.into_iter().fold(Vec3::splat(f32::INFINITY),Vec3::min),
                    max:p.into_iter().fold(Vec3::splat(f32::NEG_INFINITY),Vec3::max),planes});
            }
        }}
        cells
    })
}
pub(super) fn cut_openings(mut pieces: Vec<Vec<Vec3>>) -> Vec<Vec<Vec3>> {
    for cell in cut_cells() {
        pieces=pieces.into_iter().flat_map(|p| {
            if (0..3).any(|axis|p.iter().all(|v|v[axis]<cell.min[axis])||p.iter().all(|v|v[axis]>cell.max[axis])) {return vec![p];}
            subtract(p,&cell.planes)
        }).collect();
    }
    pieces
}
fn subtract(mut inside: Vec<Vec3>, planes: &[(Vec3,f32)]) -> Vec<Vec<Vec3>> {
    let mut outside=Vec::new();
    for &(n,d) in planes {
        if inside.len()<3 {break;}
        let mut keep=Vec::new();let mut reject=Vec::new();
        for i in 0..inside.len() {
            let a=inside[i];let b=inside[(i+1)%inside.len()];
            let da=n.dot(a)-d;let db=n.dot(b)-d;
            if da>=0. {keep.push(a);}else{reject.push(a);}
            if (da>=0.)!=(db>=0.) {let p=a.lerp(b,da/(da-db));keep.push(p);reject.push(p);}
        }
        if reject.len()>=3 {outside.push(reject);}
        inside=keep;
    }
    outside
}
pub(super) fn setup(c:&mut Commands,m:&mut Assets<Mesh>,mats:&mut Assets<StandardMaterial>,root:Entity,skin:Handle<StandardMaterial>,rim:Handle<StandardMaterial>) {
    // Opposite, culled faces keep the gray exterior and dark interior separate
    // even where the cowl thickness tapers to zero inside the fuselage.
    let lining=mats.add(StandardMaterial {base_color:Color::srgb(0.16,0.18,0.18),perceptual_roughness:0.88,
        double_sided:false,cull_mode:Some(bevy::render::render_resource::Face::Back),..default()});
    let mut exterior=mats.get(&skin).unwrap().clone();
    exterior.double_sided=false;exterior.cull_mode=Some(bevy::render::render_resource::Face::Back);
    let exterior=mats.add(exterior);
    let recess=material(mats,Color::srgb(0.015,0.018,0.02),0.05);
    for side in [-1.,1.] {
        part(c,m,root,trunk(side),lining.clone(),Transform::default());
        part(c,m,root,raptor_mesh::paint(walls(side,true)),exterior.clone(),Transform::default());
        part(c,m,root,raptor_mesh::paint(surround(side)),exterior.clone(),Transform::default());
        let front=lip(side);
        for k in 0..4 {tube(c,m,root,rim.clone(),front[k],front[(k+1)%4],0.022);}
        // Engine inlet housing at the far end of the duct, beyond the S-bend.
        part(c,m,root,solid(&ring(side,1.),Vec3::Z*0.015),recess.clone(),Transform::default());
    }
}
