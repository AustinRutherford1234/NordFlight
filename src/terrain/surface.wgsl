#import bevy_pbr::{forward_io::{VertexOutput,FragmentOutput},mesh_view_bindings::view,pbr_fragment::pbr_input_from_standard_material,pbr_functions::{apply_pbr_lighting,main_pass_post_lighting_processing,alpha_discard}}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> detail:vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var albedo:texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var albedo_sampler:sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var normals:texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var normal_sampler:sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(105) var cover_map:texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(106) var cover_sampler:sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(107) var cover_lookup:texture_2d<u32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(111) var cover_classes:texture_2d_array<u32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(112) var<uniform> palette:array<vec4<f32>,8>;
fn material_for_cover(id:u32)->f32{
    if id==2u || id==6u {return 1.0;}
    if id==3u {return 2.0;}
    if id==4u {return 4.0;}
    if id==5u {return 5.0;}
    if id==7u {return 3.0;}
    return 0.0;
}
// Color is reconstructed from geography, never from terrain triangle size.
// Derivatives use continuous lon/lat rather than fract() at degree boundaries.
fn geographic_surface(lonlat:vec2<f32>)->vec4<f32>{
    let lon=lonlat.x+12.711111;let lat=lonlat.y+58.428333;
    let coordinates=vec2<f32>(lonlat.x,-lonlat.y);
    let dx=dpdx(coordinates);let dy=dpdy(coordinates);
    let west=i32(floor(lon));let south=i32(floor(lat));
    let key=vec2<i32>((west+180+360)%360,clamp(89-south,0,179));
    let layer=i32(textureLoad(cover_lookup,key,0).r)-1;
    // All derivatives above execute even in coarse fallback areas.
    if layer<0 {return vec4<f32>(0.0,0.0,0.0,-1.0);}
    let size=vec2<f32>(textureDimensions(cover_map));
    let scale=(size-vec2<f32>(4.0))/size;
    let uv=(vec2<f32>(lon-f32(west),1.0-(lat-f32(south)))*scale)+vec2<f32>(2.0)/size;
    var color=vec3<f32>(0.0);
    let class_size=vec2<f32>(textureDimensions(cover_classes));
    let class_scale=class_size-vec2<f32>(4.0);
    let texel=vec2<f32>(lon-f32(west),1.0-(lat-f32(south)))*class_scale+vec2<f32>(1.5);
    let cell=clamp(vec2<i32>(floor(texel)),vec2<i32>(0),vec2<i32>(class_size)-vec2<i32>(2));
    let footprint=max(length(dx*class_scale),length(dy*class_scale));
    let far=smoothstep(0.75,2.0,footprint);
    if far>0.0 {color=textureSampleGrad(cover_map,cover_sampler,uv,layer,dx*scale,dy*scale).rgb;}
    if footprint>=2.0 {
        let id=textureLoad(cover_classes,cell,layer,0).r;
        return vec4<f32>(color,material_for_cover(id));
    }
    let ids=vec4<u32>(textureLoad(cover_classes,cell,layer,0).r,
        textureLoad(cover_classes,cell+vec2<i32>(1,0),layer,0).r,
        textureLoad(cover_classes,cell+vec2<i32>(0,1),layer,0).r,
        textureLoad(cover_classes,cell+vec2<i32>(1,1),layer,0).r);
    if all(ids==vec4<u32>(ids.x)) {
        return vec4<f32>(mix(palette[min(ids.x,7u)].rgb,color,far),material_for_cover(ids.x));
    }
    // Edge-preserving reconstruction: limit blending to a narrow band around
    // actual class boundaries, instead of interpolating over mesh triangles.
    let f=fract(texel);
    let weights=vec4<f32>((1.0-f.x)*(1.0-f.y),f.x*(1.0-f.y),(1.0-f.x)*f.y,f.x*f.y);
    // Combine matching labels before sharpening. Four taps suffice; no
    // eight-biome loop or unrelated palette entries are evaluated per pixel.
    let mx=select(vec4<f32>(0.0),vec4<f32>(1.0),ids==vec4<u32>(ids.x));
    let my=select(vec4<f32>(0.0),vec4<f32>(1.0),ids==vec4<u32>(ids.y));
    let mz=select(vec4<f32>(0.0),vec4<f32>(1.0),ids==vec4<u32>(ids.z));
    let mw=select(vec4<f32>(0.0),vec4<f32>(1.0),ids==vec4<u32>(ids.w));
    let ones=vec4<f32>(1.0);
    let raw=vec4<f32>(dot(weights,mx),dot(weights,my),dot(weights,mz),dot(weights,mw));
    let counts=vec4<f32>(dot(ones,mx),dot(ones,my),dot(ones,mz),dot(ones,mw));
    let sharpened=smoothstep(vec4<f32>(0.15),vec4<f32>(0.85),raw)/counts;
    let normalized=sharpened/max(dot(sharpened,ones),0.00001);
    let reconstructed=palette[min(ids.x,7u)].rgb*normalized.x+palette[min(ids.y,7u)].rgb*normalized.y
        +palette[min(ids.z,7u)].rgb*normalized.z+palette[min(ids.w,7u)].rgb*normalized.w;
    let dominant=select(select(ids.x,ids.y,raw.y>raw.x),select(ids.z,ids.w,raw.w>raw.z),max(raw.z,raw.w)>max(raw.x,raw.y));
    color=mix(reconstructed,color,far);
    return vec4<f32>(color,material_for_cover(dominant));
}
fn hash(p:vec2<f32>)->f32 {
    var q=fract(vec3<f32>(p.x,p.y,p.x)*vec3<f32>(0.1031,0.1030,0.0973));
    q+=dot(q,q.yzx+33.33);return fract((q.x+q.y)*q.z);
}
fn offset(p:vec2<f32>)->vec2<f32>{return vec2<f32>(hash(p),hash(p+vec2<f32>(17.0,31.0)))*32.0;}
fn stochastic(uv:vec2<f32>,layer:i32,dx:vec2<f32>,dy:vec2<f32>)->vec3<f32>{
    // Triangular patch blending; variance correction reduces contrast loss.
    // This is the Gaussian approximation, not full histogram LUT remapping.
    let skew=vec2<f32>(uv.x-uv.y*0.57735027,uv.y*1.15470054);
    let cell=floor(skew);let f=fract(skew);
    var w=vec3<f32>(1.0-f.x-f.y,f.x,f.y);
    var a=cell;var b=cell+vec2<f32>(1.0,0.0);var c=cell+vec2<f32>(0.0,1.0);
    if f.x+f.y>1.0 {w=vec3<f32>(f.x+f.y-1.0,1.0-f.x,1.0-f.y);a=cell+vec2<f32>(1.0);b=cell+vec2<f32>(0.0,1.0);c=cell+vec2<f32>(1.0,0.0);}
    let x=textureSampleGrad(albedo,albedo_sampler,uv+offset(a),layer,dx,dy).rgb;
    let y=textureSampleGrad(albedo,albedo_sampler,uv+offset(b),layer,dx,dy).rgb;
    let z=textureSampleGrad(albedo,albedo_sampler,uv+offset(c),layer,dx,dy).rgb;
    return clamp((x*w.x+y*w.y+z*w.z-vec3<f32>(0.214))/sqrt(dot(w,w))+vec3<f32>(0.214),vec3<f32>(0.01),vec3<f32>(1.0));
}
@fragment fn fragment(in:VertexOutput,@builtin(front_facing) front:bool)->FragmentOutput{
    var pbr=pbr_input_from_standard_material(in,front);
    let distance=length(view.world_position-in.world_position.xyz);
    // Stable regional metre coordinates. Geographic conversion is supplied
    // by mesh workers; the fragment path needs only multiply/add operations.
    let ground=in.uv*vec2<f32>(0.5236,1.0)*(detail.x*0.01745329252);
    let uv=ground*0.125;let dx=dpdx(uv);let dy=dpdy(uv);
    // Stop stochastic patch work before its eight-metre detail becomes
    // subpixel. Otherwise small triangular patches sparkle during movement.
    let footprint=max(length(dx),length(dy));
    let resolved=1.0-smoothstep(0.03,0.25,footprint);
    let near=(1.0-smoothstep(350.0,1800.0,distance))*detail.z*resolved;
    let unscaled_body=vec3<f32>(ground.x,in.uv_b.y,ground.y);
    let surface=geographic_surface(in.uv);
    let body=unscaled_body*0.125;
    let body_dx=dpdx(body);let body_dy=dpdy(body);
    let steep=max(in.uv_b.x,0.0);
    let rock=smoothstep(0.06,0.22,steep);
    let snow=smoothstep(detail.w-100.0,detail.w+100.0,unscaled_body.y)*(1.0-smoothstep(0.20,0.50,steep));
    var layer=0;
    if steep>0.12 {layer=2;}
    if unscaled_body.y>detail.w {layer=5;}
    if surface.a>=0.0 {
        layer=i32(surface.a);
        var base=surface.rgb;
        // Real water masks remain authoritative; don't whiten lake beds.
        if layer!=4 {
            base=mix(base,palette[3].rgb,rock);
            base=mix(base,palette[5].rgb,snow);
            if rock>0.5 {layer=2;}
            if snow>0.5 {layer=5;}
        }
        pbr.material.base_color=vec4<f32>(base,pbr.material.base_color.a);
    }
    if near>0.001 {
        var a=stochastic(uv,layer,dx,dy);
        var nr=textureSampleGrad(normals,normal_sampler,uv,layer,dx,dy);
        if layer==2 {
            let n=abs(normalize(cross(body_dx,body_dy)));var w=n*n*n*n;w/=max(w.x+w.y+w.z,0.0001);
            // Only steep rock pays for the extra two projections.
            let side_x=textureSampleGrad(albedo,albedo_sampler,body.zy,2,body_dx.zy,body_dy.zy).rgb;
            let side_z=textureSampleGrad(albedo,albedo_sampler,body.xy,2,body_dx.xy,body_dy.xy).rgb;
            a=a*w.y+side_x*w.x+side_z*w.z;
        }
        // Geographic palette colors are reconstructed in linear space. Detail is centered on
        // linear 0.214 (sRGB 0.5), maintaining consistent far-terrain brightness.
        let variation=clamp(a/vec3<f32>(0.214),vec3<f32>(0.65),vec3<f32>(1.4));
        pbr.material.base_color=vec4<f32>(pbr.material.base_color.rgb*mix(vec3<f32>(1.0),variation,near*0.28),pbr.material.base_color.a);
        pbr.material.perceptual_roughness=mix(0.95,clamp(nr.a,0.65,1.0),near);
        let tangent=normalize(cross(pbr.N,vec3<f32>(0.001,0.0,1.0)));let bitangent=cross(pbr.N,tangent);
        pbr.N=normalize(pbr.N+(tangent*(nr.r*2.0-1.0)+bitangent*(nr.g*2.0-1.0))*near*0.22);
        pbr.world_normal=pbr.N;
    }
    pbr.material.base_color=alpha_discard(pbr.material,pbr.material.base_color);
    var out:FragmentOutput;out.color=apply_pbr_lighting(pbr);out.color=main_pass_post_lighting_processing(pbr,out.color);return out;
}
