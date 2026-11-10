//! Nearby crossed 2D foliage cards, one mesh per cell and one shared material.
use crate::{environment::{self,ground_cover::{self,CELL}},physics::{Flight,Vehicle},world::{self,WorldObject}};
use bevy::{asset::RenderAssetUsages,math::{DQuat,DVec3},mesh::Indices,prelude::*,
    render::render_resource::{Extent3d,PrimitiveTopology,TextureDimension,TextureFormat},
    tasks::{AsyncComputeTaskPool,Task,block_on,poll_once}};
use std::{collections::{HashMap,HashSet},hash::{Hash,Hasher},sync::Arc};
#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash)]struct Key(i32,i32,u64);
struct Built {mesh:Mesh,anchor:DVec3}
struct Cell {object:Option<(Entity,Handle<Mesh>)>,vertices:usize}
struct Pending {key:Key,entity:Entity,mesh:Handle<Mesh>,vertices:usize}
#[derive(Resource)]pub(crate) struct Grass {
    cells:HashMap<Key,Cell>,jobs:HashMap<Key,Task<Built>>,wanted:HashSet<Key>,pending:Option<Pending>,
    material:Handle<StandardMaterial>,image:Handle<Image>,enabled:bool,
}
#[derive(Resource,Clone,Default)]struct Ready(crate::terrain_ready::TerrainReady);
#[derive(Default)]
struct StreamScratch {
    tiles:Vec<super::Tile>,shared_tiles:Arc<Vec<super::Tile>>,wanted:Vec<Key>,retired:Vec<Key>,missing:Vec<Key>,
}
pub struct GrassPlugin;
impl Plugin for GrassPlugin{
    fn build(&self,app:&mut App){
        let ready=Ready::default();app.insert_resource(ready.clone())
            .add_systems(PostStartup,setup).add_systems(Update,update.in_set(crate::FlightStage::World).after(super::update));
        if let Some(render)=app.get_sub_app_mut(bevy::render::RenderApp){
            render.insert_resource(ready).add_systems(bevy::render::Render,check
                .in_set(bevy::render::RenderSystems::PrepareAssets)
                .after(bevy::render::render_asset::prepare_assets::<bevy::render::mesh::RenderMesh>));
        }
    }
}
fn check(ready:Res<Ready>,meshes:Res<bevy::render::render_asset::RenderAssets<bevy::render::mesh::RenderMesh>>,
    images:Res<bevy::render::render_asset::RenderAssets<bevy::render::texture::GpuImage>>,
    materials:Res<bevy::render::erased_render_asset::ErasedRenderAssets<bevy::pbr::PreparedMaterial>>){
    ready.0.poll_all(|id|meshes.get(id).is_some(),|id|images.get(id).is_some(),|id|materials.get(id).is_some());
}
/// Original procedural coverage atlas: left grass, right broadleaf undergrowth.
/// Color comes from the planet palette in mesh vertices. No downloaded assets.
fn texture()->Image{
    let (w,h)=(256usize,128usize);let mut base=vec![255;w*h*4];
    for y in 0..h{for x in 0..w{
        let u=(x%128)as f32/127.;let v=1.-y as f32/127.;let mut coverage=0f32;
        if x<128{
            for blade in 0..7{
                let k=blade as f32;
                let length=0.50+environment::hash(blade,7,91)as f32*0.47;
                if v>length{continue;}
                let t=v/length;
                let root=0.34+k*0.05;
                let bend=(k-3.)*0.055*t*t;
                let width=(0.018+environment::hash(blade,2,71)as f32*0.012)*(1.-t).sqrt();
                coverage=coverage.max(((width-(u-root-bend).abs())*127.+0.5).clamp(0.,1.));
            }
        }else{
            coverage=((0.012-(u-0.5-0.025*v*v).abs())*127.+0.5).clamp(0.,1.)*(v<0.93)as u8 as f32;
            for leaf in 0..6{
                let level=0.18+leaf as f32*0.115;
                for side in [-1.,1.]{
                    let cx=0.5+side*(0.11-0.006*leaf as f32);let cy=level+0.045;
                    let xx=(u-cx)/0.125;let yy=(v-cy-side*(u-cx)*0.5)/0.048;
                    coverage=coverage.max(((1.-xx*xx-yy*yy)*5.).clamp(0.,1.));
                }
            }
        }
        base[(y*w+x)*4+3]=(coverage*255.).round()as u8;
    }}
    let mut bytes=Vec::new();let(mut width,mut height)=(w,h);let mut levels=0;
    loop{
        bytes.extend_from_slice(&base);levels+=1;
        if width==1&&height==1{break;}
        let(nw,nh)=((width/2).max(1),(height/2).max(1));let mut next=vec![255;nw*nh*4];
        for y in 0..nh{for x in 0..nw{
            let alpha=(0..2).flat_map(|dy|(0..2).map(move|dx|(dy,dx)))
                .map(|(dy,dx)|base[(((y*2+dy).min(height-1)*width+(x*2+dx).min(width-1))*4)+3]as u32).sum::<u32>();
            next[(y*nw+x)*4+3]=(alpha/4)as u8;
        }}
        base=next;width=nw;height=nh;
    }
    let mut image=Image::new_uninit(Extent3d{width:w as u32,height:h as u32,depth_or_array_layers:1},
        TextureDimension::D2,TextureFormat::Rgba8UnormSrgb,RenderAssetUsages::RENDER_WORLD);
    image.data=Some(bytes);image.texture_descriptor.mip_level_count=levels;
    image.sampler=bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor{
        mag_filter:bevy::image::ImageFilterMode::Linear,min_filter:bevy::image::ImageFilterMode::Linear,
        mipmap_filter:bevy::image::ImageFilterMode::Linear,anisotropy_clamp:4,..default()});image
}
fn surface(world:&environment::EnvironmentSnapshot,x:f64,z:f64,tiles:&[super::Tile],cache:&mut HashMap<(i32,i32),DVec3>)->Option<f64>{
    let tile=tiles.iter().find(|t|x>=t.x as f64&&x<=(t.x+t.size)as f64&&z>=t.z as f64&&z<=(t.z+t.size)as f64)?;
    let step=tile.size/64;
    let ix=((x-tile.x as f64)/step as f64).floor().clamp(0.,63.)as i32;
    let iz=((z-tile.z as f64)/step as f64).floor().clamp(0.,63.)as i32;
    let(x0,z0)=(tile.x+ix*step,tile.z+iz*step);
    let mut corner=|x:i32,z:i32|*cache.entry((x,z)).or_insert_with(||environment::point(x as f64,z as f64,world.height_xz(x as f64,z as f64),world.preset.planet.radius));
    let(a,b,c,d)=(corner(x0,z0),corner(x0+step,z0),corner(x0+step,z0+step),corner(x0,z0+step));
    let(u,v)=((x-x0 as f64)/step as f64,(z-z0 as f64)/step as f64);
    Some(if u+v<=1.{a.y*(1.-u-v)+b.y*u+d.y*v}else{b.y*(1.-v)+d.y*(1.-u)+c.y*(u+v-1.)})
}
fn build(world:&environment::EnvironmentSnapshot,key:Key,density:f64,tiles:&[super::Tile])->Built{
    let anchor=environment::point((key.0*CELL+CELL/2)as f64,(key.1*CELL+CELL/2)as f64,0.,world.preset.planet.radius);
    let mut heights=HashMap::new();
    let mut tufts=ground_cover::placements(world,key.0,key.1,density);
    // Stable spatially scattered thinning, rather than keeping just one side
    // of a cell when the geometric budget is reached.
    if tufts.len()>1024{
        tufts.sort_by_cached_key(|t|environment::hash(t.position.x.floor()as i32,t.position.z.floor()as i32,
            world.preset.seed.wrapping_add(0x72616e6b)).to_bits());tufts.truncate(1024);
    }
    // Two quads per tuft, capped at 8192 vertices. Compact indices preserve
    // every triangle while halving the index upload and GPU index storage.
    let vertices=tufts.len()*8;
    let(mut positions,mut normals,mut colors,mut uv,mut indices)=(Vec::with_capacity(vertices),
        Vec::with_capacity(vertices),Vec::with_capacity(vertices),Vec::with_capacity(vertices),Vec::<u16>::with_capacity(tufts.len()*12));
    for mut tuft in tufts{
        // Match the published mesh's actual triangle interpolation. Cosmetic
        // roots must not float above or disappear below a coarse render LOD.
        if let Some(y)=surface(world,tuft.position.x,tuft.position.z,tiles,&mut heights){tuft.position.y=y-0.04;}
        let up=tuft.position.normalize();let tangent=DQuat::from_rotation_arc(DVec3::Y,up);
        for angle in [tuft.heading,tuft.heading+std::f64::consts::FRAC_PI_2]{
            let right=tangent*DVec3::new(angle.cos(),0.,angle.sin())*tuft.width as f64*0.5;
            let bottom=tuft.position;let top=bottom+up*tuft.height as f64;let base=positions.len()as u16;
            let u=if tuft.broadleaf{0.5}else{0.};
            for (p,tex,shade)in [(bottom-right,[u+0.015,1.],0.65),(bottom+right,[u+0.485,1.],0.65),
                (top+right,[u+0.485,0.],1.1),(top-right,[u+0.015,0.],1.1)]{
                positions.push((p-anchor).as_vec3().to_array());normals.push(up.as_vec3().to_array());
                colors.push([tuft.color[0]*shade,tuft.color[1]*shade,tuft.color[2]*shade,1.]);uv.push(tex);
            }
            indices.extend([base,base+1,base+2,base,base+2,base+3]);
        }
    }
    Built{anchor,mesh:Mesh::new(PrimitiveTopology::TriangleList,RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION,positions).with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL,normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR,colors).with_inserted_attribute(Mesh::ATTRIBUTE_UV_0,uv)
        .with_inserted_indices(Indices::U16(indices))}
}
fn stamp(world:&environment::EnvironmentSnapshot,density:f64)->u64{
    let mut h=std::collections::hash_map::DefaultHasher::new();
    (&world.preset.planet.id,world.revision,world.preset.seed,world.preset.planet.radius.to_bits(),density.to_bits()).hash(&mut h);h.finish()
}
fn layout_key(mut key:Key,tiles:&[super::Tile])->Key{
    let mut h=std::collections::hash_map::DefaultHasher::new();let(x,z)=(key.0*CELL,key.1*CELL);
    for tile in tiles.iter().filter(|t|t.x<x+CELL&&t.x+t.size>x&&t.z<z+CELL&&t.z+t.size>z){tile.hash(&mut h);}
    key.2^=h.finish();key
}
fn keys_into(position:DVec3,agl:f64,distance:f64,density:f64,stamp:u64,keys:&mut Vec<Key>){
    keys.clear();
    if density<=0.||distance<=0.||agl>=distance{return;}
    let radius=(distance*distance-agl.max(0.).powi(2)).sqrt();
    let(cx,cz)=((position.x/CELL as f64).floor()as i32,(position.z/CELL as f64).floor()as i32);
    let n=(radius/CELL as f64).ceil()as i32;
    for z in -n..=n{for x in -n..=n{if ((x*x+z*z)as f64).sqrt()*CELL as f64<=radius{keys.push(Key(cx+x,cz+z,stamp));}}}
    keys.sort_by_key(|k|(k.0-cx).pow(2)+(k.1-cz).pow(2));keys.truncate(29);
}
#[cfg(test)]
fn keys(position:DVec3,agl:f64,distance:f64,density:f64,stamp:u64)->Vec<Key>{
    let mut keys=Vec::new();keys_into(position,agl,distance,density,stamp,&mut keys);keys
}
fn setup(mut c:Commands,mut images:ResMut<Assets<Image>>,mut materials:ResMut<Assets<StandardMaterial>>){
    if !environment::real(){return;}
    let image=images.add(texture());
    let material=materials.add(StandardMaterial{base_color_texture:Some(image.clone()),alpha_mode:AlphaMode::Mask(0.5),
        // Both sides share the bent-up normal; flipping it would blacken backs.
        double_sided:false,cull_mode:None,perceptual_roughness:1.,reflectance:0.,..default()});
    c.insert_resource(Grass{cells:HashMap::new(),jobs:HashMap::new(),wanted:HashSet::new(),pending:None,
        material,image,enabled:std::env::var_os("NORD_GRASS_OFF").is_none()});
}
fn update(mut c:Commands,f:Res<Flight>,graphics:Res<environment::graphics::Graphics>,state:Option<ResMut<Grass>>,
    mut meshes:ResMut<Assets<Mesh>>,ready:Res<Ready>,terrain:Option<Res<super::Landscape>>,mut scratch:Local<StreamScratch>){
    let Some(mut state)=state else{return;};let world=environment::active();
    let density=graphics.0.grass_density;
    let agl=f.agl();let active=state.enabled&&f.vehicle!=Vehicle::Spacecraft&&density>0.&&agl<graphics.0.grass_distance;
    if !active&&state.cells.is_empty()&&state.jobs.is_empty()&&state.pending.is_none(){state.wanted.clear();return;}
    let StreamScratch{tiles,shared_tiles,wanted,retired,missing}=&mut *scratch;
    tiles.clear();
    if active && let Some(terrain)=terrain.as_deref(){
        tiles.extend(terrain.tiles.keys().filter(|t|
        (t.x as f64-f.position.x).max(0.).max(f.position.x-(t.x+t.size)as f64)<graphics.0.grass_distance+CELL as f64
        &&(t.z as f64-f.position.z).max(0.).max(f.position.z-(t.z+t.size)as f64)<graphics.0.grass_distance+CELL as f64).copied());
    }
    // Each tile key is unique, so an unstable sort gives the same order
    // without needing scratch storage for a stable sort.
    tiles.sort_unstable_by_key(|t|(t.size,t.x,t.z));
    // Workers share an immutable snapshot. Refresh it only when the nearby
    // published terrain changes; keep all main-thread scratch allocations.
    if tiles.as_slice()!=shared_tiles.as_slice(){*shared_tiles=Arc::new(tiles.clone());}
    if active {
        keys_into(f.position,agl,graphics.0.grass_distance,density,stamp(world,density),wanted);
        for key in wanted.iter_mut(){*key=layout_key(*key,tiles);}
    }else{wanted.clear();}
    // Reuse the set allocation and split the field borrows instead of cloning
    // the entire wanted set just to cancel obsolete jobs.
    {
        let Grass{wanted:wanted_set,jobs,..}=&mut *state;
        wanted_set.clear();wanted_set.extend(wanted.iter().copied());
        jobs.retain(|key,_|wanted_set.contains(key));
    }
    if state.pending.as_ref().is_some_and(|p|!state.wanted.contains(&p.key)){
        let p=state.pending.take().unwrap();c.entity(p.entity).despawn();meshes.remove(p.mesh.id());ready.0.clear();
    }
    retired.clear();retired.extend(state.cells.keys().filter(|k|!state.wanted.contains(k)).copied().take(4));
    for key in retired.iter(){if let Some(cell)=state.cells.remove(key){if let Some((e,m))=cell.object{c.entity(e).despawn();meshes.remove(m.id());}}}
    if !retired.is_empty(){return;}
    if state.pending.is_some()&&ready.0.ready(){
        let p=state.pending.take().unwrap();c.entity(p.entity).insert(Visibility::Inherited);
        state.cells.insert(p.key,Cell{object:Some((p.entity,p.mesh)),vertices:p.vertices});ready.0.clear();
    }
    if state.pending.is_none(){
        let completed=state.jobs.iter_mut().find_map(|(key,task)|block_on(poll_once(task)).map(|b|(*key,b)));
        if let Some((key,built))=completed{
            state.jobs.remove(&key);let vertices=built.mesh.count_vertices();
            if vertices==0{state.cells.insert(key,Cell{object:None,vertices:0});}
            else{
                let mesh=meshes.add(built.mesh);
                let entity=c.spawn((Mesh3d(mesh.clone()),MeshMaterial3d(state.material.clone()),Visibility::Hidden,
                    Transform::from_translation(world::local(built.anchor,&f)).with_rotation(world::local_rot(DQuat::IDENTITY,&f)),
                    WorldObject{position:built.anchor,rotation:DQuat::IDENTITY},bevy::light::NotShadowCaster,bevy::light::NotShadowReceiver)).id();
                ready.0.request(vec![mesh.id()]);ready.0.dependencies(vec![state.image.id()],vec![state.material.id().untyped()]);
                state.pending=Some(Pending{key,entity,mesh,vertices});
            }
        }
    }
    missing.clear();missing.extend(wanted.iter().copied().filter(|k|!state.cells.contains_key(k)&&!state.jobs.contains_key(k)
        &&!state.pending.as_ref().is_some_and(|p|p.key==*k)).take(2usize.saturating_sub(state.jobs.len())));
    for key in missing.drain(..){
        let world=Arc::clone(world);let tiles=Arc::clone(shared_tiles);
        state.jobs.insert(key,AsyncComputeTaskPool::get().spawn(async move{build(&world,key,density,&tiles)}));
    }
}
pub(crate) fn loading_resources(f:&Flight,state:Option<&Grass>)->Option<crate::loading::Resources>{
    let mut out=crate::loading::Resources::default();
    if !environment::real()||f.vehicle==Vehicle::Spacecraft{return Some(out);}
    let state=state?;
    for key in &state.wanted{
        let cell=state.cells.get(key)?;
        if let Some((_,mesh))=&cell.object{out.meshes.push(mesh.id());}
    }
    if !out.meshes.is_empty(){out.images.push(state.image.id());out.materials.push(state.material.id().untyped());}
    Some(out)
}
pub fn report(state:Option<&Grass>)->serde_json::Value{
    state.map_or(serde_json::Value::Null,|s|serde_json::json!({"enabled":s.enabled,"cells":s.cells.len(),"wanted":s.wanted.len(),"jobs":s.jobs.len(),
        "pending":s.pending.is_some(),"draw_batches":s.cells.values().filter(|c|c.object.is_some()).count(),
        "vertices":s.cells.values().map(|c|c.vertices).sum::<usize>(),
        "active_mesh_payload_bytes":s.cells.values().map(|c|c.vertices*51).sum::<usize>()}))
}
pub fn study(){
    let world=environment::active();let mut cases=Vec::new();
    for(x,z)in [(-8000.,0.),(-512.,0.),(512.,-1600.),(6500.,3200.),(292778.,301221.)]{
        let key=Key((x/CELL as f64).floor()as i32,(z/CELL as f64).floor()as i32,0);
        let b=build(world,key,0.65,&[]);let h=world.height_xz(x,z);
        cases.push(serde_json::json!({"x":x,"z":z,"height":h,"vertices":b.mesh.count_vertices(),
            "tufts":b.mesh.count_vertices()/8,"land_cover":format!("{:?}",world.raw(environment::point(x,z,h,world.preset.planet.radius)).1),
            "source_bytes":b.mesh.attributes().map(|(_,v)|v.get_bytes().len()).sum::<usize>()+b.mesh.indices().unwrap().len()*2}));
    }
    std::fs::create_dir_all("tests/grass").unwrap();
    std::fs::write("tests/grass/placement.json",serde_json::to_string_pretty(&cases).unwrap()).unwrap();
    println!("{cases:?}");
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn cells_are_bounded_and_above_ground_range_grass_is_free(){
        for distance in [112.,160.,192.]{assert!(keys(DVec3::ZERO,0.,distance,1.,0).len()<=29);}
        assert!(keys(DVec3::ZERO,113.,112.,1.,0).is_empty());
        assert!(keys(DVec3::ZERO,0.,112.,0.,0).is_empty());
        let a=keys(DVec3::new(-1.,0.,-1.),0.,112.,1.,1);assert_eq!(a[0],Key(-1,-1,1));
    }
    #[test]fn adjoining_terrain_does_not_rebuild_unchanged_grass_cells(){
        let a=super::super::Tile{x:0,z:0,size:2048};let b=super::super::Tile{x:2048,z:0,size:2048};
        let key=Key(0,0,1);
        assert_eq!(layout_key(key,&[a]),layout_key(key,&[a,b]));
        assert_ne!(layout_key(key,&[a]),layout_key(key,&[super::super::Tile{x:0,z:0,size:4096}]));
    }
    #[test]fn feet_match_both_triangles_of_the_actual_rendered_terrain_grid(){
        let world=environment::EnvironmentSnapshot::legacy();let tile=super::super::Tile{x:12288,z:6144,size:2048};
        let anchor=environment::point(13312.,7168.,0.,world.preset.planet.radius);
        let mesh=super::super::mesh::terrain_mesh(tile.x as f64,tile.z as f64,2048.,64,anchor);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(p))=mesh.attribute(Mesh::ATTRIBUTE_POSITION)else{panic!()};
        let mut cache=HashMap::new();let(i,j)=(16usize,8usize);
        let(a,b,c,d)=(p[j*65+i][1]as f64+anchor.y,p[j*65+i+1][1]as f64+anchor.y,
            p[(j+1)*65+i+1][1]as f64+anchor.y,p[(j+1)*65+i][1]as f64+anchor.y);
        for(u,v)in [(0.2,0.3),(0.8,0.6)]{
            let y=surface(&world,12288.+(i as f64+u)*32.,6144.+(j as f64+v)*32.,&[tile],&mut cache).unwrap();
            let rendered=if u+v<=1.{a*(1.-u-v)+b*u+d*v}else{b*(1.-v)+d*(1.-u)+c*(u+v-1.)};
            assert!((y-rendered).abs()<0.002);
        }
    }
    #[test]fn cards_have_valid_geometry_and_source_mask_has_mipmaps(){
        let world=environment::EnvironmentSnapshot::legacy();let b=build(&world,Key(200,100,1),0.65,&[]);
        let n=b.mesh.count_vertices();assert!(n>0&&n<=8192&&n%8==0);
        assert!(b.mesh.indices().unwrap().iter().all(|i|i<n));
        let image=texture();assert_eq!(image.texture_descriptor.mip_level_count,9);
        let data=image.data.unwrap();assert!(data[..256*128*4].chunks_exact(4).any(|p|p[3]==0));
        assert!(data[..256*128*4].chunks_exact(4).any(|p|p[3]==255));
        assert!(stamp(&world,0.65)!=stamp(&world,0.85));
    }
    #[test]fn disabling_grass_rejects_a_gpu_ready_but_obsolete_cell(){
        let mut app=App::new();app.add_plugins((MinimalPlugins,bevy::asset::AssetPlugin::default()))
            .init_resource::<Assets<Mesh>>().insert_resource(Ready::default())
            .insert_resource(environment::graphics::Graphics(environment::GraphicsSettings::default()))
            .add_systems(Update,update);
        let mut f=Flight::approach();let w=environment::active();let h=w.height_xz(12800.,6400.);
        f.position=environment::point(12800.,6400.,h+2.,w.preset.planet.radius);app.insert_resource(f);
        app.insert_resource(Grass{cells:HashMap::new(),jobs:HashMap::new(),wanted:HashSet::new(),pending:None,
            material:Handle::default(),image:Handle::default(),enabled:true});
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(15);
        while app.world().resource::<Grass>().pending.is_none(){
            app.update();assert!(std::time::Instant::now()<deadline);std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let entity=app.world().resource::<Grass>().pending.as_ref().unwrap().entity;
        assert_eq!(*app.world().get::<Visibility>(entity).unwrap(),Visibility::Hidden);
        app.world().resource::<Ready>().0.poll_all(|_|true,|_|true,|_|true);
        app.world_mut().resource_mut::<environment::graphics::Graphics>().0.grass_density=0.;app.update();
        let grass=app.world().resource::<Grass>();
        assert!(grass.pending.is_none()&&grass.jobs.is_empty()&&grass.wanted.is_empty());
        assert!(app.world().get_entity(entity).is_err());
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(),0);
    }
}
