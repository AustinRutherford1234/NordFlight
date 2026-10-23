//! Photo-guided intake presentation. Internal routing is an approximation;
//! continuous S-bends conceal the engine face without an inlet blanking plate.
use super::*;

pub(super) const SLICES: usize = 64;
#[derive(Component)]
struct DuctLining;
pub(super) fn body_sections()->Vec<(f64,f64,f64,f64)> {
    let sections=sections(Aircraft::F22);let last=*sections.last().unwrap();
    sections.windows(2).flat_map(|p|(0..4).map(move|j| {
        let t=j as f64/4.;
        (p[0].0+(p[1].0-p[0].0)*t,p[0].1+(p[1].1-p[0].1)*t,
         p[0].2+(p[1].2-p[0].2)*t,p[0].3+(p[1].3-p[0].3)*t)
    })).chain(std::iter::once(last)).collect()
}
pub(super) fn body_point((z,w,h,y):(f64,f64,f64,f64),k:usize)->Vec3 {
    let theta=k as f32/64.*std::f32::consts::TAU;let x=theta.cos();
    forebody(Vec3::new(w as f32*x,y as f32+if theta.sin()>=0.{h as f32*raptor_mesh::upper(x)}else{crate::aircraft::raptor::lower_profile(x,z as f32)},z as f32),w as f32)
}
fn body_section(z:f32)->(f32,f32,f32,f32) {
    static SECTIONS:std::sync::OnceLock<Vec<(f64,f64,f64,f64)>>=std::sync::OnceLock::new();
    let s=SECTIONS.get_or_init(||sections(Aircraft::F22));
    let pair=s.windows(2).find(|s|z>=s[0].0 as f32&&z<=s[1].0 as f32).unwrap();
    let t=(z-pair[0].0 as f32)/(pair[1].0-pair[0].0)as f32;
    (z,pair[0].1 as f32*(1.-t)+pair[1].1 as f32*t,pair[0].2 as f32*(1.-t)+pair[1].2 as f32*t,pair[0].3 as f32*(1.-t)+pair[1].3 as f32*t)
}
pub(super) fn body_height(x:f32,z:f32,top:bool)->f32 {
    let (_,w,h,cy)=body_section(z);
    let point=|u|forebody(Vec3::new(w*u,cy+if top{h*raptor_mesh::upper(u)}else{crate::aircraft::raptor::lower_profile(u,z)},z),w);
    let mut lo=0.;let mut hi=1.;
    for _ in 0..24 {
        let u=(lo+hi)*0.5;if point(u).x>x.abs(){hi=u}else{lo=u}
    }
    point((lo+hi)*0.5).y
}
/// Shape the forebody to the inboard splitter rather than leaving the old
/// wide cheek in front of the tucked-back aperture. This deforms a closed
/// loft; an upstream boolean alone would leave an open hole in the nose.
pub(super) fn forebody(mut p:Vec3,section_width:f32)->Vec3 {
    let along=crate::aircraft::bays::smooth((p.z+5.4)/1.1)
        *(1.-crate::aircraft::bays::smooth((p.z+2.68)/0.58));
    let q=lip(1.);
    let inner=q[3].x+(q[0].x-q[3].x)*(p.y-q[3].y)/(q[0].y-q[3].y);
    let above=crate::aircraft::bays::smooth((p.y-q[0].y)/0.30);
    let width=inner.max(0.3).min(section_width)*(1.-above)+section_width*above;
    p.x*=1.-along*(1.-width/section_width.max(0.01));p
}
pub(super) fn lip(side: f32) -> [Vec3; 4] {
    let mut p=[Vec3::new(side*1.00,0.24,-4.30), Vec3::new(side*2.15,0.24,-2.68),
        Vec3::new(side*1.60,-0.80,-2.33), Vec3::new(side*0.45,-0.80,-3.95)];
    // One swept mouth plane. The old independently authored fourth corner
    // twisted the narrow rim enough to produce alternating sliver normals.
    let n=(p[1]-p[0]).cross(p[3]-p[0]);
    p[2].z=p[0].z-(n.x*(p[2].x-p[0].x)+n.y*(p[2].y-p[0].y))/n.z;
    p
}
pub(super) fn ring(side: f32, u: f32) -> [Vec3; 4] {
    // The entrance first runs aft at almost constant height. The S-bend
    // then turns inboard and rises above the main bay, behind the side-bay lip.
    const ROUTE: [[f32;6];10] = [
        [0.,1.30,-0.28,-3.315,0.575,0.52],
        [0.08,1.26,-0.28,-2.50,0.47,0.50],
        [0.14,0.90,0.22,-1.75,0.38,0.24],
        [0.16,0.92,0.30,-1.45,0.40,0.24],
        [0.22,0.90,0.38,-0.90,0.49,0.20],
        [0.30,0.84,0.42,-0.40,0.49,0.13],
        [0.34,0.83,0.42,-0.05,0.42,0.11],
        [0.45,0.83,0.42,0.50,0.38,0.11],
        [0.70,0.88,0.40,2.40,0.36,0.10],
        [1.,0.93,-0.12,5.02,0.52,0.40],
    ];
    let u=u.clamp(0.,1.);
    let i=ROUTE.windows(2).position(|p| u<=p[1][0]).unwrap();
    let t=(u-ROUTE[i][0])/(ROUTE[i+1][0]-ROUTE[i][0]);
    let value=|k:usize| {
        let tangent=|j:usize| {
            if j==0 {(ROUTE[1][k]-ROUTE[0][k])/(ROUTE[1][0]-ROUTE[0][0])}
            else if j==ROUTE.len()-1 {(ROUTE[j][k]-ROUTE[j-1][k])/(ROUTE[j][0]-ROUTE[j-1][0])}
            else {(ROUTE[j+1][k]-ROUTE[j-1][k])/(ROUTE[j+1][0]-ROUTE[j-1][0])}
        };
        let h=ROUTE[i+1][0]-ROUTE[i][0];
        (2.*t*t*t-3.*t*t+1.)*ROUTE[i][k]+(t*t*t-2.*t*t+t)*h*tangent(i)
            +(-2.*t*t*t+3.*t*t)*ROUTE[i+1][k]+(t*t*t-t*t)*h*tangent(i+1)
    };
    let x=value(1);let y=value(2);let z=value(3);let w=value(4);let h=value(5);
    let first=lip(1.);
    let base=[Vec3::new(0.725,0.24,-3.315),Vec3::new(1.875,0.24,-3.315),
        Vec3::new(1.875,-0.80,-3.315),Vec3::new(0.725,-0.80,-3.315)];
    let fade=(1.-u/0.28).max(0.).powi(2);
    std::array::from_fn(|k| {
        let dx=if k==0||k==3{-w}else{w};let dy=if k<2{h}else{-h};
        let mut p=Vec3::new(x+dx,y+dy,z)+(first[k]-base[k])*fade;
        p.x*=side;p
    })
}
fn outer_ring(side: f32, u: f32) -> [Vec3; 4] {
    let mut inner=ring(side,u);
    // Exterior cowl and internal flow path have different jobs. Retain the
    // broad shoulder aft to the wing root instead of wrapping the fuselage
    // around the rapidly contracting internal duct like a separate horn.
    let shoulder=1.-crate::aircraft::bays::smooth((u-0.30)/0.14);
    let along=crate::aircraft::bays::smooth(u/0.30);
    inner[1].x+=(side*(lip(1.)[1].x*(1.-along)+2.15*along)-inner[1].x)*shoulder;
    inner[1].y+=(0.24-0.28*along-inner[1].y)*shoulder;
    // Thin coated lip, with its boundary also used to cut the body loft.
    let blend=(1.-u/0.08).clamp(0.,1.);
    let width=0.025*blend*blend;
    let across=(inner[1]-inner[0]+inner[2]-inner[3])*0.5;
    let up=(inner[0]-inner[3]+inner[1]-inner[2])*0.5;
    let n=across.cross(up);let gx=-n.x/n.z;let gy=-n.y/n.z;
    std::array::from_fn(|k| {
        let dx=side*if k==0||k==3{-width}else{width};let dy=if k<2{width}else{-width};
        // Expand within the swept mouth plane, then bevel forward. A fixed
        // world-Z offset inverted the bevel on the swept outboard edge.
        inner[k]+Vec3::new(dx,dy,gx*dx+gy*dy-0.012*blend*blend)
    })
}
const OUTER:usize=7;
fn outer_perimeter(side:f32,u:f32)->[Vec3;OUTER] {
    let mut q=outer_ring(side,u);
    let shell=0.012*crate::aircraft::bays::smooth(u/0.10);
    q[0].x-=side*shell;
    q[2]+=Vec3::new(side*shell,-shell,0.);
    q[3]+=Vec3::new(-side*shell,-shell,0.);
    // A finite roof shell keeps the triangulated exterior clear of the
    // curved duct between stations, including the tighter entrance bend.
    q[0].y+=shell;
    let mut crest=ring(side,u)[1];crest.y+=0.012;
    // A separate roof crest encloses the raised duct, then slopes down to
    // the wing shoulder. Raising the entire outboard edge made a broad hump;
    // a four-corner roof pulled straight to the shoulder cut through the duct.
    crest=q[1].lerp(crest,crate::aircraft::bays::smooth(u/0.10));
    crest.y=crest.y.max(ring(side,u)[1].y);
    // Bury the inner edges of the upper/lower fairings in the actual closed
    // body loft. Preserve the lip and duct corners between those anchors;
    // raising the lower lip itself would cut through the airflow passage.
    let top_x=q[0].x*0.45;let bottom_x=q[3].x*0.45;
    let top=Vec3::new(top_x,body_height(top_x,q[0].z,true)-0.06,q[0].z);
    let bottom=Vec3::new(bottom_x,body_height(bottom_x,q[3].z,false)+0.06,q[3].z);
    [top,q[0],crest,q[1],q[2],q[3],bottom]
}
// The six tetrahedra and the four side quads use the same face diagonals.
// A single plane per twisted quad used to leave slivers of fuselage in the lip.
const TETS: [[usize;4];6]=[[0,1,2,6],[0,2,3,6],[0,3,7,6],[0,7,4,6],[0,4,5,6],[0,5,1,6]];
fn vertices(side:f32,j:usize,outer:bool)->[Vec3;8] {
    let at=|u|if outer{outer_ring(side,u)}else{ring(side,u)};
    let a=at(j as f32/SLICES as f32);let b=at((j+1)as f32/SLICES as f32);
    [a[0],a[1],a[2],a[3],b[0],b[1],b[2],b[3]]
}
const PERIMETER:usize=28;
fn rounded(side:f32,u:f32)->[Vec3;PERIMETER] {
    let q=ring(side,u);let radius=0.11+0.05*u;
    std::array::from_fn(|i| {
        let k=i/7;let t=(i%7)as f32/6.;
        let a=q[k].lerp(q[(k+3)%4],radius);let b=q[k].lerp(q[(k+1)%4],radius);
        a*(1.-t).powi(2)+q[k]*(2.*t*(1.-t))+b*t*t
    })
}
fn walls(side: f32, outer: bool) -> Mesh {
    let mut v=Vec::new();let mut ix=Vec::new();let mut ao_uv=Vec::new();let mut normals=Vec::new();
    let source_normals=if outer{cowl_normals(side)}else{Vec::new()};
    for j in 0..SLICES {
        let a=if outer{outer_perimeter(side,j as f32/SLICES as f32).to_vec()}else{rounded(side,j as f32/SLICES as f32).to_vec()};
        let b=if outer{outer_perimeter(side,(j+1)as f32/SLICES as f32).to_vec()}else{rounded(side,(j+1)as f32/SLICES as f32).to_vec()};
        for k in 0..a.len() {
            let n=(k+1)%a.len();let mut q=[a[k],b[k],b[n],a[n]];
            let mut uv=[[j as f32/SLICES as f32,0.],[(j+1)as f32/SLICES as f32,0.],[(j+1)as f32/SLICES as f32,1.],[j as f32/SLICES as f32,1.]];
            if (side>0.)!=outer {if outer{q.swap(1,3);}else{q.reverse();}}
            if (side>0.)!=outer {uv.reverse();}
            if outer {
                let mut nq=[source_normals[j][k][0],source_normals[j+1][k][0],source_normals[j+1][k][1],source_normals[j][k][1]];
                if side<0.{nq.swap(1,3);}
                for ids in [[0,1,2],[0,2,3]] {
                    let t=ids.map(|i|q[i]);let ns=ids.map(|i|nq[i]);let before=v.len();
                    for p in cut_skin_access(Aircraft::F22,cut_body(vec![t.to_vec()])) {polygon_into(&mut v,&mut ix,&p);}
                    for p in &v[before..] {
                        let w=closest_weights(*p,t);
                        normals.push((ns[0]*w.x+ns[1]*w.y+ns[2]*w.z).normalize_or(ns[0]).to_array());
                    }
                }
            }else{quad(&mut v,&mut ix,q);ao_uv.extend(uv);}
        }
    }
    if outer {
        // Close the splitter fairing to the original mouth without covering
        // the aperture. These two triangles are also used by its cut volume.
        let q=outer_perimeter(side,0.);
        for mut t in [[q[1],q[5],q[6]],[q[1],q[6],q[0]]] {
            if side<0.{t.swap(1,2);}
            let before=v.len();
            for p in cut_skin_access(Aircraft::F22,cut_body(vec![t.to_vec()])){polygon_into(&mut v,&mut ix,&p);}
            normals.extend(std::iter::repeat_n((t[1]-t[0]).cross(t[2]-t[0]).normalize_or_zero().to_array(),v.len()-before));
        }
    }
    let mut mesh=if outer{let mut m=mesh(v,ix);m.insert_attribute(Mesh::ATTRIBUTE_NORMAL,normals);m}else{raptor_mesh::smooth(mesh(v,ix))};
    if !outer {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1,ao_uv);
        if let Some(bevy::mesh::VertexAttributeValues::Float32x3(p))=mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            let colors=p.iter().map(|p|{let depth=((p[2]+4.3)/9.3).clamp(0.,1.);let g=0.97-0.18*depth;[g,g,g,1.]}).collect::<Vec<_>>();
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR,colors);
        }
    }
    mesh
}
fn cowl_normals(side:f32)->Vec<[[Vec3;2];OUTER]> {
    // Compute on the authored panels, then interpolate through clipping.
    // Averaging normals after clipping made arbitrary tiny fragments visible.
    let mut n=vec![[[Vec3::ZERO;2];OUTER];SLICES+1];
    for j in 0..SLICES {
        let a=outer_perimeter(side,j as f32/SLICES as f32);let b=outer_perimeter(side,(j+1)as f32/SLICES as f32);
        for k in 0..OUTER {
            let next=(k+1)%OUTER;let q=[a[k],b[k],b[next],a[next]];
            let refs=[(j,0),(j+1,0),(j+1,1),(j,1)];
            for t in [[0,1,2],[0,2,3]] {
                let normal=(q[t[1]]-q[t[0]]).cross(q[t[2]]-q[t[0]])*side;
                for i in t {let (s,v)=refs[i];n[s][k][v]+=normal;}
            }
        }
    }
    for station in &mut n{for panel in station{for normal in panel{*normal=normal.normalize_or_zero();}}}n
}
pub(super) fn trunk(side:f32)->Mesh {walls(side,false)}
struct BodyFace {p:[Vec3;3],n:[Vec3;3],min:Vec3,max:Vec3}
fn body_faces()->&'static [BodyFace] {
    static FACES:std::sync::OnceLock<Vec<BodyFace>>=std::sync::OnceLock::new();
    FACES.get_or_init(|| {
        let sections=body_sections();
        let points=sections.iter().flat_map(|s|(0..64).map(move|k|body_point(*s,k))).collect::<Vec<_>>();
        let mut normals=vec![Vec3::ZERO;points.len()];let mut triangles=Vec::new();
        for j in 0..sections.len()-1 {for k in 0..64 {
            let q=[j*64+k,j*64+(k+1)%64,(j+1)*64+(k+1)%64,(j+1)*64+k];
            for t in [[q[0],q[1],q[2]],[q[0],q[2],q[3]]] {
                let [a,b,c]=t.map(|i|points[i]);let n=(b-a).cross(c-a);
                for i in t{normals[i]+=n;}triangles.push(t);
            }
        }}
        for n in &mut normals{*n=n.normalize_or_zero();}
        triangles.into_iter().filter_map(|t| {
            let p=t.map(|i|points[i]);
            if p.iter().all(|p|p.z< -4.6)||p.iter().all(|p|p.z>0.7){return None;}
            Some(BodyFace {p,n:t.map(|i|normals[i]),min:p.into_iter().fold(Vec3::splat(f32::INFINITY),Vec3::min),max:p.into_iter().fold(Vec3::splat(f32::NEG_INFINITY),Vec3::max)})
        }).collect()
    })
}
// Closest point and barycentrics on a triangle, including its edge regions.
fn closest_weights(p:Vec3,[a,b,c]:[Vec3;3])->Vec3 {
    let ab=b-a;let ac=c-a;let ap=p-a;let d1=ab.dot(ap);let d2=ac.dot(ap);
    if d1<=0.&&d2<=0.{return Vec3::X;}
    let bp=p-b;let d3=ab.dot(bp);let d4=ac.dot(bp);
    if d3>=0.&&d4<=d3{return Vec3::Y;}
    let vc=d1*d4-d3*d2;
    if vc<=0.&&d1>=0.&&d3<=0.{let v=d1/(d1-d3);return Vec3::new(1.-v,v,0.);}
    let cp=p-c;let d5=ab.dot(cp);let d6=ac.dot(cp);
    if d6>=0.&&d5<=d6{return Vec3::Z;}
    let vb=d5*d2-d1*d6;
    if vb<=0.&&d2>=0.&&d6<=0.{let w=d2/(d2-d6);return Vec3::new(1.-w,0.,w);}
    let va=d3*d6-d5*d4;
    if va<=0.&&d4-d3>=0.&&d5-d6>=0.{let w=(d4-d3)/((d4-d3)+(d5-d6));return Vec3::new(0.,1.-w,w);}
    let inv=1./(va+vb+vc);let v=vb*inv;let w=vc*inv;Vec3::new(1.-v-w,v,w)
}
pub(super) fn seam_normals(mut mesh:Mesh)->Mesh {
    let p=mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap();
    let n=mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().as_float3().unwrap();
    let normals=p.iter().zip(n).map(|(p,n)| {
        let p=Vec3::from_array(*p);let original=Vec3::from_array(*n);
        if !(-4.45..=0.50).contains(&p.z)||p.x.abs()<0.12{return original.to_array();}
        let mut nearest=0.08f32.powi(2);let mut shared=original;
        for face in body_faces().iter().filter(|f|(0..3).all(|a|p[a]>=f.min[a]-0.08&&p[a]<=f.max[a]+0.08)) {
            let w=closest_weights(p,face.p);let q=face.p[0]*w.x+face.p[1]*w.y+face.p[2]*w.z;
            let d=q.distance_squared(p);
            if d<nearest {nearest=d;shared=(face.n[0]*w.x+face.n[1]*w.y+face.n[2]*w.z).normalize_or(original);}
        }
        let blend=(1.-nearest.sqrt()/0.08).clamp(0.,1.);
        original.lerp(shared,blend).normalize_or(original).to_array()
    }).collect::<Vec<_>>();mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,normals);mesh
}
fn surround(side:f32)->Mesh {
    let mut v=Vec::new();let mut ix=Vec::new();
    let outer=outer_ring(side,0.);let inner=rounded(side,0.);
    let boundary: [Vec3;PERIMETER]=std::array::from_fn(|i| {
        let k=i/7;let t=(i%7)as f32/6.;
        if t<0.5 {outer[k].lerp(outer[(k+3)%4],0.11*(1.-2.*t))}
        else{outer[k].lerp(outer[(k+1)%4],0.11*(2.*t-1.))}
    });
    for k in 0..PERIMETER {
        let n=(k+1)%PERIMETER;
        let mut q=[boundary[k],boundary[n],inner[n],inner[k]];
        if side<0. {q.reverse();}
        quad(&mut v,&mut ix,q);
    }
    mesh(v,ix)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forebody_leaves_the_inlet_apertures_visible_from_ahead() {
        let skin=super::super::hull_mesh(Aircraft::F22,false);
        let p=skin.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap();
        let indices=skin.indices().unwrap().iter().collect::<Vec<_>>();
        for side in [-1.,1.] {
            let q=lip(side);
            for x in [0.2,0.4,0.6,0.8] {for y in [0.2,0.4,0.6,0.8] {
                let target=q[3].lerp(q[2],x).lerp(q[0].lerp(q[1],x),y);
                let origin=target-Vec3::Z*8.;
                for t in indices.chunks_exact(3) {
                    let [a,b,c]=[t[0],t[1],t[2]].map(|i|Vec3::from_array(p[i]));
                    let e=b-a;let f=c-a;let h=Vec3::Z.cross(f);let det=e.dot(h);
                    if det.abs()<1e-8{continue;}
                    let s=origin-a;let u=s.dot(h)/det;
                    let k=s.cross(e);let v=Vec3::Z.dot(k)/det;let distance=f.dot(k)/det;
                    assert!(!(u>0.&&v>0.&&u+v<1.&&distance>0.&&distance<7.99),"nose cheek obstructs aperture {target:?} at {:?}",origin+Vec3::Z*distance);
                }
            }}
        }
        inspect_aperture();
    }
    fn inspect_aperture() {
        use super::super::raptor_bay::tests::{base_matrix,belongs};
        let mut flight=Flight::runway();crate::aircraft::select(&mut flight,Aircraft::F22);flight.paused=true;flight.gear=true;flight.gear_extension=1.;
        let mut app=App::new();app.init_resource::<Assets<Mesh>>().init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>().init_resource::<Time>().insert_resource(flight)
            .add_systems(Startup,super::super::setup)
            .add_systems(Update,(super::super::update,raptor_bay::update,super::super::raptor_main_bay::update,super::super::raptor_detail::update).chain());app.update();
        let w=app.world_mut();let root=w.query::<(Entity,&Root)>().iter(w).find(|(_,r)|r.0==Aircraft::F22).unwrap().0;
        let inverse=base_matrix(w,root).inverse();
        for x in [1.35,1.42,1.48,1.54] {for y in [-0.10,-0.20,-0.30] {
            let origin=Vec3::new(0.,-0.15,-32.8);let d=(Vec3::new(x,y,-1.87)-origin).normalize();let mut hits=Vec::new();
            for (e,m,mat) in w.query::<(Entity,&Mesh3d,&MeshMaterial3d<StandardMaterial>)>().iter(w) {
                if !belongs(w,e,root){continue;}
                if w.get::<Store>(e).is_some_and(|s|s.1>=16){continue;}
                let mesh=w.resource::<Assets<Mesh>>().get(&m.0).unwrap();let material=w.resource::<Assets<StandardMaterial>>().get(&mat.0).unwrap();
                if material.alpha_mode!=AlphaMode::Opaque{continue;}
                let matrix=inverse*base_matrix(w,e);let vertices=mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap();
                let indices=mesh.indices().unwrap().iter().collect::<Vec<_>>();
                for t in indices.chunks_exact(3) {
                    let [a,b,c]=[t[0],t[1],t[2]].map(|i|matrix.transform_point3(Vec3::from_array(vertices[i])));
                    let edge=b-a;let f=c-a;let h=d.cross(f);let det=edge.dot(h);
                    if det.abs()<1e-8||(material.cull_mode==Some(bevy::render::render_resource::Face::Back)&&det<0.){continue;}
                    let s=origin-a;let u=s.dot(h)/det;let k=s.cross(edge);let v=d.dot(k)/det;let distance=f.dot(k)/det;
                    if u>=0.&&v>=0.&&u+v<=1.&&distance>0. {
                        hits.push((distance,e,origin+d*distance,material.base_color,vertices.len(),w.get::<DuctLining>(e).is_some()));
                    }
                }
            }
            hits.sort_by(|a,b|a.0.total_cmp(&b.0));
            assert!(hits.first().is_some_and(|h|h.5),"hardware protrudes ahead of duct lining at ray {x},{y}: {:?}",hits.first());
            if std::env::var_os("NORD_APERTURE_INSPECT").is_some(){println!("aperture ray {x},{y}: {:?}",hits.into_iter().take(3).collect::<Vec<_>>());}
        }}
    }
    #[test]
    fn entrance_runs_aft_before_rising_and_shoulders_reach_wing_root() {
        let center=|u|ring(1.,u).into_iter().sum::<Vec3>()/4.;
        let a=center(0.);let b=center(0.08);let raised=center(0.22);
        assert!(b.z-a.z>0.75,"entrance must run aft before bending");
        assert!((b.y-a.y).abs()<0.05,"entrance must not turn straight upward");
        assert!((b.x-a.x).abs()<0.10,"entrance must not immediately collapse inboard");
        assert!(raised.z-a.z>1.9&&raised.y-a.y>0.30);
        let q=lip(1.);
        assert!(q[0].y-q[3].y>0.95);
        assert!(q[2].x<q[1].x&&q[3].x<q[0].x,"mouth must cant outward toward its top");
        for (top,bottom) in [(0,3),(1,2)] {
            let cant=((q[top].x-q[bottom].x)/(q[top].y-q[bottom].y)).atan().to_degrees();
            assert!((25. ..=30.).contains(&cant),"photo-guided mouth cant: {cant}");
        }
        assert!(q[2].z>q[1].z+0.20,"lower lip must sweep aft in side view");
        assert!(outer_ring(1.,0.16)[1].x>2.10,"exterior shoulder must remain broad above the internal bend");
        if let Ok(path)=std::env::var("NORD_INLET_GEOMETRY_JSON") {
            let samples=(0..=128).map(|i| {
                let u=i as f32/128.;serde_json::json!({"u":u,"inner":ring(1.,u).map(|p|p.to_array()),"outer":outer_ring(1.,u).map(|p|p.to_array())})
            }).collect::<Vec<_>>();
            std::fs::write(path,serde_json::to_string_pretty(&samples).unwrap()).unwrap();
        }
    }
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
            let mouth=lip(side);
            let normal=(mouth[1]-mouth[0]).cross(mouth[3]-mouth[0]).normalize();
            for (outer,inner) in outer_ring(side,0.).into_iter().zip(ring(side,0.)) {
                // Measure bevel thickness normal to the swept/canted mouth;
                // world-Z displacement also includes its in-plane widening.
                assert!((normal.dot(outer-inner)+normal.z*0.012).abs()<1e-6);
                assert!(((outer.y-inner.y).abs()-0.025).abs()<1e-6);
            }
            let surround=surround(side);
            assert_eq!(surround.count_vertices(),PERIMETER*4);
            let coated=raptor_mesh::paint_bevel(surround.clone());
            assert_eq!(coated.count_vertices(),surround.count_vertices(),"coating must not perforate the thin corner bevels");
            assert_eq!(coated.indices().unwrap().iter().collect::<Vec<_>>(),surround.indices().unwrap().iter().collect::<Vec<_>>());
            for normal in surround.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().as_float3().unwrap() {
                assert!(normal[2]< -0.1,"surround must face forward");
            }
        }
        assert!(cut_cells().iter().all(|c|c.planes.iter().all(|(n,d)|n.is_finite()&&d.is_finite())));
    }
    #[test]
    fn clipped_join_normals_follow_the_uncut_surface() {
        let face=&body_faces()[body_faces().len()/2];
        for w in [Vec3::X,Vec3::Y,Vec3::Z,Vec3::splat(1./3.),Vec3::new(0.2,0.3,0.5)] {
            let p=face.p[0]*w.x+face.p[1]*w.y+face.p[2]*w.z;
            assert!(closest_weights(p,face.p).distance(w)<0.001);
        }
        for side in [-1.,1.] {
            let normals=cowl_normals(side);
            for j in 0..SLICES {
                let a=outer_perimeter(side,j as f32/SLICES as f32);let b=outer_perimeter(side,(j+1)as f32/SLICES as f32);
                for k in 0..OUTER {
                    let next=(k+1)%OUTER;
                    let geometric=(b[k]-a[k]).cross(b[next]-a[k])*side;
                    if geometric.length_squared()<1e-12{continue;}
                    let normal=normals[j][k][0]+normals[j+1][k][0]+normals[j+1][k][1];
                    assert!(normal.dot(geometric)>0.,"reversed cowl normal {side} {j} {k}");
                }
            }
            let m=seam_normals(walls(side,true));
            let p=m.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap();
            let n=m.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().as_float3().unwrap();
            assert_eq!(p.len(),n.len());
            assert!(n.iter().all(|n|{let n=Vec3::from_array(*n);n.is_finite()&&(n.length()-1.).abs()<0.001}));
        }
    }
    #[test]
    fn inboard_fairings_anchor_inside_the_deformed_body_on_both_sides() {
        let inside=|p:Vec3|body_cells().iter().any(|c|c.planes.iter().all(|(n,d)|n.dot(p)>=*d-0.000001));
        for side in [-1.,1.] {for j in 0..=SLICES {
            let q=outer_perimeter(side,j as f32/SLICES as f32);
            assert!(inside(q[0]),"upper fairing disconnected at {side} {j}: {:?}",q[0]);
            assert!(inside(q[6]),"lower fairing disconnected at {side} {j}: {:?}",q[6]);
        }}
    }
    #[test]
    fn body_trim_uses_the_rendered_loft_triangles() {
        let mut checked=[0;2];
        for pair in body_sections().windows(2) {
            if pair[0].0< -4.4||pair[1].0> -0.5{continue;}
            for k in 0..64 {
                let q=[body_point(pair[0],k),body_point(pair[0],k+1),body_point(pair[1],k+1),body_point(pair[1],k)];
                for [a,b,c] in [[q[0],q[1],q[2]],[q[0],q[2],q[3]]] {
                    let center=(a+b+c)/3.;
                    if center.x.abs()<0.15 {continue;}
                    let n=(b-a).cross(c-a).normalize();
                    let inside=|p:Vec3|body_cells().iter().any(|c|c.planes.iter().all(|(n,d)|n.dot(p)>=*d));
                    assert!(inside(center-n*0.001),"untrimmed body interior at {center:?}");
                    assert!(!inside(center+n*0.001),"trim reaches outside rendered body at {center:?}");
                    checked[usize::from(center.y>pair[0].3 as f32)]+=1;
                }
            }
        }
        assert!(checked.into_iter().all(|n|n>100));
    }
    #[test]
    fn cowl_cutouts_meet_the_rendered_triangles_without_body_slivers() {
        let area=|pieces:Vec<Vec<Vec3>>|pieces.iter().map(|p|(1..p.len()-1)
            .map(|i|(p[i]-p[0]).cross(p[i+1]-p[0]).length()*0.5).sum::<f32>()).sum::<f32>();
        let mut exposed=0;
        for side in [-1.,1.] {for j in 0..8 {
            let start=outer_perimeter(side,j as f32/SLICES as f32);
            let end=outer_perimeter(side,(j+1)as f32/SLICES as f32);
            for k in 0..OUTER {
                let next=(k+1)%OUTER;
                let mut p=[start[k],end[k],end[next],start[next]];
                if side<0.{p.swap(1,3);}
                for t in [[0,1,2],[0,2,3]] {
                    let [a,b,c]=t.map(|k|p[k]);
                    if (b-a).cross(c-a).length_squared()<1e-12{continue;}
                    let centre=(a+b+c)/3.;let n=(b-a).cross(c-a).normalize();
                    let u=(b-a).normalize()*0.0001;let v=n.cross(u);
                    let sample=|sign|vec![centre+n*sign*0.001-u-v,centre+n*sign*0.001+u-v,centre+n*sign*0.001+v];
                    let outside=sample(1.);let expected=area(vec![outside.clone()]);
                    assert!(area(cut_openings(vec![sample(-1.)]))<expected*0.05,"body remains inside cowl at side {side} panel {k} slice {j}: {centre:?}");
                    // A separate internal duct can cross this exterior face.
                    // Only a point outside the complete union retains skin.
                    let p=centre+n*0.001;
                    let occupied=cut_cells().iter().any(|c|c.planes.iter().all(|(n,d)|n.dot(p)>=*d));
                    if !occupied {
                        exposed+=1;
                        assert!(area(cut_openings(vec![outside]))>expected*0.95,"gap outside cowl at {j}");
                    }
                }
            }
        }}
        assert!(exposed>20,"insufficient exposed cowl boundary coverage");
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
            assert_eq!(positions.len(),SLICES*PERIMETER*4);
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
                    if p.z>=-3.37+crate::aircraft::bays::OFFSET.z && p.z<=-0.04+crate::aircraft::bays::OFFSET.z {
                        assert!(p.x.abs()<crate::aircraft::bays::BACK||p.y> -0.14+crate::aircraft::bays::OFFSET.y,"side bay: {p:?}");
                    }
                    if p.x.abs()<crate::aircraft::main_bay::HALF&&p.z>=crate::aircraft::main_bay::FORE && p.z<=crate::aircraft::main_bay::AFT {assert!(p.y>crate::aircraft::main_bay::ROOF,"main bay: {p:?}");}
                    let (well,width)=raptor_detail::well_layout(2);
                    if p.z>=well.z-1.85&&p.z<=well.z+0.65&&p.x.abs()>=well.x.abs()-width {
                        assert!(p.y>0.28,"main gear pocket: {p:?}");
                    }
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
        let remainder=cut_openings(vec![plate]);
        let area=remainder.iter().map(|p|(1..p.len()-1).map(|i|(p[i]-p[0]).cross(p[i+1]-p[0]).length()*0.5).sum::<f32>()).sum::<f32>();
        assert!(area<1e-8,"uncut stream-tube area {area}, {remainder:?}");
        let outside=vec![Vec3::new(3.,0.,-3.),Vec3::new(3.1,0.,-3.),Vec3::new(3.,0.1,-3.)];
        assert_eq!(cut_openings(vec![outside.clone()]),vec![outside]);
    }
    #[test]
    fn exterior_shoulder_does_not_cross_inside_the_duct() {
        let inside=|p:Vec3|inner_cells().iter().any(|c|c.planes.iter().all(|(n,d)|n.dot(p)>*d+0.00001));
        let mut original_crossings=0;
        for j in 0..SLICES {
            let a=outer_perimeter(1.,j as f32/SLICES as f32);
            let b=outer_perimeter(1.,(j+1)as f32/SLICES as f32);
            for k in 0..OUTER {
                let n=(k+1)%OUTER;let q=[a[k],b[k],b[n],a[n]];
                for [i,j,k] in [[0,1,2],[0,2,3]] {
                    original_crossings+=usize::from(inside((q[i]+q[j]+q[k])/3.));
                }
            }
        }
        assert_eq!(original_crossings,0,"outer envelope must enclose the duct before trimming");
        for side in [-1.,1.] {
            let mesh=walls(side,true);
            let p=mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap();
            let indices=mesh.indices().unwrap().iter().collect::<Vec<_>>();
            assert!(indices.len()>300,"exterior must be retained");
            for tri in indices.chunks_exact(3) {
                let c=tri.iter().map(|i|Vec3::from_array(p[*i])).sum::<Vec3>()/3.;
                assert!(!inside(c),"exterior cowl inside duct at {c:?}");
            }
            // The enclosure must be solid as seen from above, too. Merely
            // subtracting an intersecting roof clears the duct but opens skin.
            let covered=|point:Vec3|indices.chunks_exact(3).any(|tri| {
                let [a,b,c]=[tri[0],tri[1],tri[2]].map(|i|Vec3::from_array(p[i]));
                let u=b-a;let v=c-a;let w=point-a;let n=u.cross(v);
                if n.length_squared()<1e-14||n.normalize().dot(w).abs()>0.00005{return false;}
                let d=u.dot(u)*v.dot(v)-u.dot(v).powi(2);
                if d.abs()<1e-14{return false;}
                let s=(w.dot(u)*v.dot(v)-w.dot(v)*u.dot(v))/d;
                let t=(w.dot(v)*u.dot(u)-w.dot(u)*u.dot(v))/d;
                s>=-0.001&&t>=-0.001&&s+t<=1.001
            });
            for j in 0..SLICES {
                let a=outer_perimeter(side,j as f32/SLICES as f32);
                let b=outer_perimeter(side,(j+1)as f32/SLICES as f32);
                for k in 0..3 {
                let mut q=[a[k],b[k],b[k+1],a[k+1]];
                if side<0. {q.swap(1,3);} // reverse winding, retain the shared diagonal
                for tri in [[q[0],q[1],q[2]],[q[0],q[2],q[3]]] {
                    // Buried fairing geometry is intentionally removed. Every
                    // exposed part of the authored roof must remain covered.
                    for p in cut_skin_access(Aircraft::F22,cut_body(vec![tri.to_vec()])) {
                        for t in 1..p.len()-1 {
                            if (p[t]-p[0]).cross(p[t+1]-p[0]).length_squared()<1e-12{continue;}
                            assert!(covered((p[0]+p[t]+p[t+1])/3.),"missing exposed roof triangle {side} {j} {k}");
                        }
                    }
                }
                }
            }
        }
    }
    #[test]
    fn s_bend_obscures_the_entire_engine_face_from_the_lip() {
        let r=lip(1.);let rear=ring(1.,1.);
        let mesh=trunk(1.);
        let p=mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap();
        let indices=mesh.indices().unwrap();
        let triangles=indices.iter().collect::<Vec<_>>();
        let sample=|q:[Vec3;4],s:f32,t:f32| q[0].lerp(q[1],s).lerp(q[3].lerp(q[2],s),t);
        let aperture=rounded(1.,0.);
        let open=|p:Vec3| {
            let cross=(0..PERIMETER).map(|i| {
                let a=aperture[i].truncate();let b=aperture[(i+1)%PERIMETER].truncate();
                (b-a).perp_dot(p.truncate()-a)
            }).collect::<Vec<_>>();
            cross.iter().all(|v|*v>=-1e-7)||cross.iter().all(|v|*v<=1e-7)
        };
        let mut tested=0;
        for x in 0..=6 {for y in 0..=6 {for ex in 0..=4 {for ey in 0..=4 {
            let a=sample(r,0.02+0.96*x as f32/6.,0.02+0.96*y as f32/6.);
            // The old rectangular corner samples now lie in the solid lip
            // fillets. Test rays entering the actual rounded aperture.
            if !open(a){continue;}tested+=1;
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
        assert!(tested>=1000,"insufficient aperture coverage: {tested}");
    }
}

// Subtract each short convex stream tube from the fuselage. Unlike bounding
// boxes this seats the swept lips exactly and leaves no solid skin in the duct.
struct CutCell {min:Vec3,max:Vec3,planes:[(Vec3,f32);4]}
fn cell(p:[Vec3;4])->Option<CutCell> {
    if (p[1]-p[0]).cross(p[2]-p[0]).dot(p[3]-p[0]).abs()<1e-12{return None;}
    let centre=p.iter().copied().sum::<Vec3>()/4.;
    let planes=[[0,1,2],[0,3,1],[0,2,3],[1,3,2]].map(|face| {
        let [a,b,c]=face.map(|k|p[k]);let mut n=(b-a).cross(c-a).normalize();
        if n.dot(centre-a)<0.{n=-n;}(n,n.dot(a))
    });
    Some(CutCell {min:p.into_iter().fold(Vec3::splat(f32::INFINITY),Vec3::min),
        max:p.into_iter().fold(Vec3::splat(f32::NEG_INFINITY),Vec3::max),planes})
}
fn body_cells()->&'static [CutCell] {
    static CELLS:std::sync::OnceLock<Vec<CutCell>>=std::sync::OnceLock::new();
    CELLS.get_or_init(|| {
        let mut cells=Vec::new();
        for pair in body_sections().windows(2) {
            if pair[1].0< -4.5||pair[0].0>5.1{continue;}
            for k in 0..64 {
                let p=[Vec3::new(0.,pair[0].3 as f32,pair[0].0 as f32),body_point(pair[0],k),body_point(pair[0],k+1),
                    Vec3::new(0.,pair[1].3 as f32,pair[1].0 as f32),body_point(pair[1],k),body_point(pair[1],k+1)];
                for t in [[0,1,2,5],[0,1,5,4],[0,4,5,3]] {if let Some(c)=cell(t.map(|i|p[i])){cells.push(c);}}
            }
        }
        cells
    })
}
fn cut_body(pieces:Vec<Vec<Vec3>>)->Vec<Vec<Vec3>> {cut_cells_from(pieces,body_cells())}
#[cfg(test)]
fn inner_cells()-> &'static [CutCell] {
    static CELLS:std::sync::OnceLock<Vec<CutCell>>=std::sync::OnceLock::new();
    CELLS.get_or_init(|| {
        let mut cells=Vec::new();
        for side in [-1.,1.] {for j in 0..SLICES {
            let v=vertices(side,j,false);
            for tet in TETS {
                let p=tet.map(|k|v[k]);let centre=p.iter().copied().sum::<Vec3>()/4.;
                let planes=[[0,1,2],[0,3,1],[0,2,3],[1,3,2]].map(|face| {
                    let [a,b,c]=face.map(|k|p[k]);
                    let mut n=(b-a).cross(c-a).normalize();
                    if n.dot(centre-a)<0. {n=-n;}
                    let ids=face.map(|k|tet[k]);
                    // Keep the coincident outer skin on the duct boundary.
                    // Internal tetrahedron/section faces remain exact, so
                    // there are no slivers between subtracted volumes.
                    let wall=[[0,4,5,1],[1,5,6,2],[3,2,6,7],[0,3,7,4]]
                        .iter().any(|q|ids.iter().all(|i|q.contains(i)));
                    (n,n.dot(a)+if wall{0.0002}else{0.})
                });
                cells.push(CutCell {min:p.into_iter().fold(Vec3::splat(f32::INFINITY),Vec3::min),
                    max:p.into_iter().fold(Vec3::splat(f32::NEG_INFINITY),Vec3::max),planes});
            }
        }}
        cells
    })
}
fn cut_cells()-> &'static [CutCell] {
    static CELLS:std::sync::OnceLock<Vec<CutCell>>=std::sync::OnceLock::new();
    CELLS.get_or_init(|| {
        let mut cells=Vec::new();
        for side in [-1.,1.] {
            for j in 0..SLICES {
                let a=outer_perimeter(side,j as f32/SLICES as f32);
                let b=outer_perimeter(side,(j+1)as f32/SLICES as f32);
                let centre=a.into_iter().chain(b).sum::<Vec3>()/(OUTER*2)as f32;
                let mut faces=Vec::new();
                for k in 0..OUTER {
                    let n=(k+1)%OUTER;let mut q=[a[k],b[k],b[n],a[n]];
                    if side<0.{q.swap(1,3);}
                    faces.extend([[q[0],q[1],q[2]],[q[0],q[2],q[3]]]);
                }
                for q in [a,b] {for k in 2..OUTER{faces.push([q[1],q[k],q[(k+1)%OUTER]]);}}
                for [a,b,c] in faces {if let Some(c)=cell([a,b,c,centre]){cells.push(c);}}
            }
            let a=outer_ring(side,0.);let b=ring(side,0.);
            // The solid bevel has its own volume. A rapidly turning trunk's
            // first segment does not contain the entire rear lip perimeter;
            // cutting only that segment left coplanar body shards in the rim.
            let collar=[a[0],a[1],a[2],a[3],b[0],b[1],b[2],b[3]];
            for p in (0..SLICES).map(|j|vertices(side,j,false)).chain(std::iter::once(collar)) {
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
pub(super) fn cut_openings(pieces: Vec<Vec<Vec3>>) -> Vec<Vec<Vec3>> {
    cut_cells_from(pieces,cut_cells())
}
fn cut_cells_from(pieces: Vec<Vec<Vec3>>,cells:&[CutCell])->Vec<Vec<Vec3>> {
    pieces.into_iter().flat_map(|p| {
        // Clipping can only shrink this polygon. Reject unrelated cells once
        // before fragmenting it, keeping the exact plane order for candidates.
        let min=p.iter().copied().fold(Vec3::splat(f32::INFINITY),Vec3::min);
        let max=p.iter().copied().fold(Vec3::splat(f32::NEG_INFINITY),Vec3::max);
        let mut pieces=vec![p];
        for cell in cells.iter().filter(|c|(0..3).all(|a|max[a]>=c.min[a]&&min[a]<=c.max[a])) {
            pieces=pieces.into_iter().flat_map(|p| {
                if (0..3).any(|axis|p.iter().all(|v|v[axis]<cell.min[axis])||p.iter().all(|v|v[axis]>cell.max[axis])) {return vec![p];}
                subtract(p,&cell.planes)
            }).collect();
            if pieces.is_empty(){break;}
        }
        pieces
    }).collect()
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
pub(super) fn setup(c:&mut Commands,m:&mut Assets<Mesh>,mats:&mut Assets<StandardMaterial>,detail:&raptor_finish::Detail,root:Entity,skin:Handle<StandardMaterial>,rim:Handle<StandardMaterial>) {
    // Opposite, culled faces keep the gray exterior and dark interior separate
    // even where the cowl thickness tapers to zero inside the fuselage.
    let lining=mats.add(StandardMaterial {base_color:Color::srgb(0.61,0.64,0.62),perceptual_roughness:0.68,
        double_sided:false,cull_mode:Some(bevy::render::render_resource::Face::Back),..default()});
    let mut exterior=mats.get(&skin).unwrap().clone();
    exterior.double_sided=false;exterior.cull_mode=Some(bevy::render::render_resource::Face::Back);
    let exterior=mats.add(exterior);let exterior=detail.apply(mats,&exterior);
    let lining=detail.apply(mats,&lining);let lining=detail.duct(mats,&lining);
    let recess=material(mats,Color::srgb(0.015,0.018,0.02),0.05);
    for side in [-1.,1.] {
        for (index,(shape,material)) in [(trunk(side),lining.clone()),(raptor_mesh::paint(seam_normals(walls(side,true))),exterior.clone()),
            (raptor_mesh::smooth(raptor_mesh::paint_bevel(surround(side))),exterior.clone())].into_iter().enumerate() {
            let e=part(c,m,root,raptor_finish::uv(shape),material,Transform::default());
            c.entity(e).insert((crate::render_batch::RigidGeometry,Visibility::Inherited));
            if index==0{c.entity(e).insert(DuctLining);}
        }
        // Flush access fasteners and a fine coating break, rather than the old
        // raised tubular frame around the intake mouth.
        let front=outer_ring(side,0.);
        let mut normal=(front[1]-front[0]).cross(front[3]-front[0]).normalize();
        if normal.z>0.{normal=-normal;}
        for k in 0..4 {for j in 1..12 {
            let p=front[k].lerp(front[(k+1)%4],j as f32/12.);
            let e=part(c,m,root,Cylinder::new(0.0032,0.0008).mesh().resolution(12).build(),rim.clone(),
                Transform::from_translation(p+normal*0.0005).with_rotation(Quat::from_rotation_arc(Vec3::Y,normal)));
            // Flush fasteners do not cast a resolved shadow at this scene's
            // cascade texel size. Spherical heads produced oversized dark
            // shadow texels along an otherwise clean lip.
            c.entity(e).insert((crate::render_batch::RigidGeometry,Visibility::Inherited,bevy::light::NotShadowCaster));
        }}
        // Engine inlet housing at the far end of the duct, beyond the S-bend.
        part(c,m,root,solid(&ring(side,1.),Vec3::Z*0.015),recess.clone(),Transform::default());
    }
}
