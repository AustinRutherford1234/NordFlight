// Evaluated at the original mesh vertices, preserving the original interpolation.
#import bevy_pbr::{mesh_functions, forward_io::{Vertex, VertexOutput}, view_transformations::position_world_to_clip}
struct Parameters { eye: vec4<f32>, sun: vec4<f32>, layer: vec4<f32> }
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> earth: Parameters;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var nordic_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var nordic_sampler: sampler;
fn eased_curve(x: f32) -> f32 {
    let t = clamp(x, 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}
fn daylight(d: f32) -> f32 { return 0.16 + 0.84 * eased_curve((d + 0.10) / 0.45); }
@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(v.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(v.normal, v.instance_index);
#ifdef VERTEX_UVS_A
    out.uv = v.uv;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = v.instance_index;
#endif
    let n = normalize(v.normal);
    let eye = earth.eye.xyz;
    let height = earth.layer.y;
    let delta = eye - n * (earth.layer.x + height);
    if earth.layer.w < 0.5 {
        let view = delta / max(length(delta), 1e-20);
        let mu = abs(dot(n, view));
        let a = 1.0 - mu;
        let a2 = a * a;
        let rim = a2 * a2 * a2 * eased_curve(mu / 0.16);
        let outside = eased_curve((length(eye) - earth.layer.x - height) / 35000.0);
        var column = 0.004;
        if height < 10000.0 { column = 0.035; }
        else if height < 30000.0 { column = 0.012; }
        out.color = vec4<f32>(0.20, 0.50, 1.0, (column + rim * earth.layer.z) * outside * daylight(dot(n, earth.sun.xyz)));
    } else if earth.layer.w < 1.5 {
        let shade = daylight(dot(n, earth.sun.xyz));
        let near = eased_curve((length(eye) - earth.layer.x - 2000.0) / 12000.0);
        out.color = vec4<f32>(shade * 0.94, shade * 0.97, shade, near * eased_curve(length(delta) / 8000.0));
    } else {
        let shade = daylight(dot(n, earth.sun.xyz));
        out.color = vec4<f32>(shade, shade, shade, v.color.a);
    }
    return out;
}

// Regional detail is albedo on the globe, never a second transparent shell.
// Retain the global mosaic's broad colour/brightness and transfer only the
// Nordic image's fine detail. Both receive exactly the same solar shading.
#import bevy_pbr::{
    forward_io::FragmentOutput,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, main_pass_post_lighting_processing},
}
@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr = pbr_input_from_standard_material(in, is_front);
    if earth.layer.w > 1.5 {
        let lon_lat = vec2<f32>(in.uv.x * 360.0 - 180.0, 90.0 - in.uv.y * 180.0);
        let uv = vec2<f32>(lon_lat.x / 35.0, (75.0 - lon_lat.y) / 30.0);
        // Explicit LOD avoids nonuniform derivative sampling at the boundary.
        let dx = dpdx(uv);
        let dy = dpdy(uv);
        let size = vec2<f32>(textureDimensions(nordic_texture));
        let lod = max(0.0, log2(max(length(dx * size), length(dy * size))));
        let edge = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
        let weight = eased_curve(edge * 12.0);
        if weight > 0.0 {
            let detail = textureSampleLevel(nordic_texture, nordic_sampler, uv, lod).rgb;
            // This texture has no mip chain; use a fixed, symmetric low-pass
            // neighbourhood (~15 km), independent of camera/zoom and lighting.
            let d = vec2<f32>(0.004, 0.004);
            let low = (textureSampleLevel(nordic_texture, nordic_sampler, uv + d, 0.0).rgb
                + textureSampleLevel(nordic_texture, nordic_sampler, uv - d, 0.0).rgb
                + textureSampleLevel(nordic_texture, nordic_sampler, uv + vec2<f32>(d.x, -d.y), 0.0).rgb
                + textureSampleLevel(nordic_texture, nordic_sampler, uv + vec2<f32>(-d.x, d.y), 0.0).rgb) * 0.25;
            let ratio = clamp(detail / max(low, vec3<f32>(0.015)), vec3<f32>(0.75), vec3<f32>(1.3));
            pbr.material.base_color = vec4<f32>(pbr.material.base_color.rgb * mix(vec3<f32>(1.0), ratio, weight * 0.55), pbr.material.base_color.a);
        }
    }
    var out: FragmentOutput;
    out.color = alpha_discard(pbr.material, pbr.material.base_color);
    out.color = main_pass_post_lighting_processing(pbr, out.color);
    return out;
}
