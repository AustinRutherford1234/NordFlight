//! Static CC-BY 4.0 manilov.ap SR-71, normalized offline by scripts/sr71/prepare.py.
use super::*;
pub fn setup(c:&mut Commands,server:&AssetServer,root:Entity){
    let model=c.spawn((Name::new("SR-71 static imported airframe"),
        SceneRoot(crate::model_assets::load(server,GltfAssetLabel::Scene(0).from_asset("aircraft/sr71/static.glb"))),
        Transform::default(),Visibility::Inherited)).id();
    c.entity(root).add_child(model);
}
#[cfg(test)]mod tests {
    #[test]fn static_asset_is_embedded_normalized_and_has_no_animations(){
        let data=include_bytes!("../../../assets/aircraft/sr71/static.glb");
        assert_eq!(&data[..4],b"glTF");
        let n=u32::from_le_bytes(data[12..16].try_into().unwrap())as usize;
        let j:serde_json::Value=serde_json::from_slice(&data[20..20+n]).unwrap();
        assert!(j.get("animations").is_none_or(|a|a.as_array().unwrap().is_empty()));
        assert!(j["images"].as_array().unwrap().iter().all(|i|i.get("bufferView").is_some()));
        let mut lo=[f64::INFINITY;3];let mut hi=[f64::NEG_INFINITY;3];
        for mesh in j["meshes"].as_array().unwrap(){for p in mesh["primitives"].as_array().unwrap(){
            let a=&j["accessors"][p["attributes"]["POSITION"].as_u64().unwrap()as usize];
            for i in 0..3{lo[i]=lo[i].min(a["min"][i].as_f64().unwrap());hi[i]=hi[i].max(a["max"][i].as_f64().unwrap());}
        }}
        assert!((hi[2]-lo[2]-32.73).abs()<0.01);assert!((hi[0]-lo[0]-17.0255).abs()<0.02);
        assert!((lo[1]+2.).abs()<0.01);
    }
}
