//! World-anchored lake patches. Dry corners inherit a neighboring wet height;
//! they must never pull an elevated lake down to sea level.
use super::*;
pub(super) struct BuiltWater {
    pub near: Image,
    pub coarse: Image,
    pub bounds: Vec4,
    pub far: Vec4,
    pub lakes: Mesh,
    pub anchor: DVec3,
}
fn triangle_level(samples: [Option<f64>; 3], sea: f64) -> Option<f64> {
    // This runs twice per lake grid cell. Iterate the three stack-resident
    // corners directly instead of allocating a Vec for every triangle.
    let wet = samples.into_iter().flatten();
    if wet.clone().next().is_none() || wet.clone().all(|h| (h-sea).abs()<1.) { return None; }
    // Do not bridge two unrelated water bodies across a narrow ridge.
    let low=wet.clone().fold(f64::INFINITY,f64::min);
    let high=wet.clone().fold(f64::NEG_INFINITY,f64::max);
    (high-low<4.).then(||wet.clone().sum::<f64>()/wet.count() as f64)
}
pub(super) fn build(x:f64,z:f64)->BuiltWater {
    let n=384;let step=64.;
    let (near,bounds)=water_image(x,z,n,step);
    let (coarse,far)=water_image(x,z,256,1024.);
    let anchor=crate::environment::point(bounds.x as f64,bounds.y as f64,0.,planet_radius());
    let mut positions=vec![];let mut uv=vec![];let mut levels=vec![];let mut indices=vec![];
    let mut append_grid=|image:&Image,bounds:Vec4,n:usize,step:f64,kind:f32| {
    let bytes=image.data.as_ref().unwrap();
    let water:Vec<_>=bytes.chunks_exact(8).map(|v| {
        let h=f32::from_le_bytes(v[..4].try_into().unwrap())as f64;
        (f32::from_le_bytes(v[4..].try_into().unwrap())>0.5).then_some(h)
    }).collect();
    // Share the exact same elevation at each vertex between adjacent triangles,
    // including the dry side of the shoreline. Triangle-local averages crack.
    let mut extended=water.clone();
    for j in 0..n {for i in 0..n {
        if extended[j*n+i].is_some(){continue;}
        let mut sum=0.;let mut count=0.;
        for y in j.saturating_sub(1)..=(j+1).min(n-1){for x in i.saturating_sub(1)..=(i+1).min(n-1){
            if let Some(h)=water[y*n+x]{sum+=h;count+=1.;}
        }}
        if count>0.{extended[j*n+i]=Some(sum/count);}
    }}
    let sea=crate::environment::active().preset.planet.sea_level;
    let mut vertices=std::collections::HashMap::new();
    // Shore masks stay at full resolution. Geometry uses shared vertices and
    // twice the spacing: waves finer than these cells stay in filtered normals.
    for j in (0..n-1).step_by(2) {for i in (0..n-1).step_by(2) {
        let right=(i+2).min(n-1);let top=(j+2).min(n-1);
        for corners in [[(i,j),(i,top),(right,j)],[(right,j),(i,top),(right,top)]] {
            let Some(level)=triangle_level(corners.map(|(i,j)|water[j*n+i]),sea)else{continue;};
            for (i,j)in corners {
                let inherited=extended[j*n+i].unwrap_or(level);
                let level=if (inherited-level).abs()<4.{inherited}else{level};
                let key=(i,j,level.to_bits());
                if let Some(index)=vertices.get(&key){indices.push(*index);continue;}
                let index=positions.len()as u32;vertices.insert(key,index);indices.push(index);
                let x=bounds.x as f64+i as f64*step;let z=bounds.y as f64+j as f64*step;
                // The shader reconstructs curvature and wave displacement from
                // stable absolute x/z. Mesh positions provide conservative bounds.
                positions.push((crate::environment::point(x,z,level,planet_radius())-anchor).as_vec3().to_array());
                uv.push([x as f32,z as f32]);levels.push([level as f32,kind]);
            }
        }
    }}
    };
    append_grid(&near,bounds,n,step,1.);
    append_grid(&coarse,far,256,1024.,2.);
    // Keep a harmless degenerate triangle when no lakes exist, so readiness
    // has a GPU mesh to acknowledge on ocean-only routes.
    if positions.is_empty(){positions=vec![[0.;3];3];uv=vec![[0.;2];3];levels=vec![[0.;2];3];indices=vec![0,1,2];}

    let mut lakes=Mesh::new(PrimitiveTopology::TriangleList,RenderAssetUsages::RENDER_WORLD);
    let count=positions.len();
    lakes.insert_attribute(Mesh::ATTRIBUTE_POSITION,positions);
    lakes.insert_attribute(Mesh::ATTRIBUTE_NORMAL,vec![[0.,1.,0.];count]);
    lakes.insert_attribute(Mesh::ATTRIBUTE_COLOR,vec![[1.;4];count]);
    lakes.insert_attribute(Mesh::ATTRIBUTE_UV_0,uv);
    lakes.insert_attribute(Mesh::ATTRIBUTE_UV_1,levels);
    lakes.insert_indices(if count<=usize::from(u16::MAX)+1 {
        Indices::U16(indices.into_iter().map(|index|index as u16).collect())
    }else{Indices::U32(indices)});
    BuiltWater{near,coarse,bounds,far,lakes,anchor}
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn dry_bank_does_not_lower_lake_and_unrelated_levels_do_not_bridge(){
        assert_eq!(triangle_level([Some(140.),None,None],0.),Some(140.));
        assert_eq!(triangle_level([Some(140.),Some(141.),None],0.),Some(140.5));
        assert_eq!(triangle_level([Some(0.),Some(140.),None],0.),None);
        assert_eq!(triangle_level([Some(0.),None,None],0.),None);
    }
}
