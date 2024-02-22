#import bevy_pbr::{
    pbr_types,
    pbr_deferred_types,
    rgb9e5,
    utils,
}
#import bevy_render::view::View
#import pulse::{
    utils::{
        TWO_PI,
        rand_f,
        rand_f_pair,
        rand_range_u,
        sample_cosine_hemisphere,
        transform_direction,
        trace_ray,
        trace_shadow_ray,
        distance_sq,
        sample_direct_light,
    }, 
    scene::{
        types::{
            Ray, 
            RayHitRecord,
            Material,
            Primitive,
        },
        bindings::{
            instances,
            triangle_indices,
            triangle_data,
            materials,
            light_emission_strength_cdf,
            light_triangle_area_cdfs,
            light_mesh_areas,
            light_indices,
            scene_uniform,
            primitives,
        }
    },
}

struct PathTracerUniform {
    width: u32,
    height: u32,
    accumulation_count: u32,
}

@group(1) @binding(0) var<uniform> view: View;
@group(1) @binding(1) var deferred_prepass_texture: texture_2d<u32>;
@group(1) @binding(2) var depth_prepass_texture: texture_depth_2d;
@group(1) @binding(3) var output_texture: texture_storage_2d<rgba32float, read_write>;
@group(1) @binding(4) var<uniform> path_tracer_uniform: PathTracerUniform;

// @compute @workgroup_size(16, 16, 1)
// fn path_trace(@builtin(global_invocation_id) id: vec3<u32>) {
//     let pixel_index = id.x + id.y * u32(view.viewport.z);

//     var rng_state = pixel_index + path_tracer_uniform.previous_sample_count * 5817321u;
//     let bn_texture_offset = vec2u(0u);

//     let pixel_jitter = rand_f_pair(&rng_state);
//     var pixel_uv = (vec2<f32>(id.xy) + pixel_jitter) / view.viewport.zw;

//     // Clip position goes from -1 to 1.
//     let pixel_clip_pos = (pixel_uv * 2.0) - 1.0;
//     let ray_target = view.inverse_view_proj * vec4<f32>(pixel_clip_pos.x, -pixel_clip_pos.y, 1.0, 1.0);
//     var ray = Ray(); // Should always be kept in world space.
//     ray.origin = view.world_position;
//     ray.dir = normalize((ray_target.xyz / ray_target.w) - ray.origin);
//     let t_far = 1e30;
//     ray.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);
    
//     var throughput = vec3f(1.0);
//     var color = vec3f(0.0);
//     color = vec3f(next_bn_sample(&rng_state, bn_texture_offset, id.xy));
    
//     let depth_hard_cut: u32 = 0u;
//     for (var depth: u32 = 0u; depth < depth_hard_cut; depth += 1u){    
//         trace_ray_tlas(&ray);
//         if ray.record.t >= t_far  {
//             // Miss
//             // color += throughput * vec3<f32>(0.0, 0.7, 1.0) * 0.01;
//             // color += throughput * vec3<f32>(1.0, 1.0, 1.0) * 0.1;
//             break;
//         } else {
//             // Hit
//             let instance = instances[ray.record.instance_index];
//             let t_idx = triangle_indices[instance.index_offset + ray.record.triangle_index];
//             let t = triangle_data[instance.triangle_offset + t_idx];
//             let w = 1.0 - (ray.record.u + ray.record.v);
//             let normal = w * t.n_first + ray.record.u * t.n_second + ray.record.v * t.n_third;
//             let world_normal = normalize(transform_direction(instance.object_world, normal));
//             let world_hit_position = ray.origin + ray.record.t * ray.dir;

//             let material_index = instance.material_index;
//             var material = materials[material_index];

//             // material.perceptual_roughness = 0.7;
//             // material.base_color = vec4f(1.0, 0.0, 0.0, 1.0);
//             // material.metallic = 0.0;
//             // material.reflectance = 0.5;
//             // material.emissive = vec4(10.0, 0.0, 0.0, 0.0);

//             color += throughput * material.emissive.xyz * 1.5;
//             let sample = importance_sample_ggx_d(world_normal, -ray.dir, material, &rng_state, bn_texture_offset, id.xy);
//             throughput *= sample.reflectance;

//             let p = max(max(throughput.r, throughput.g), throughput.b);
//             if rand_f(&rng_state) > p { 
//                 break; 
//             }
//             throughput *= 1.0 / p;

//             ray.dir = normalize(sample.wi);
//             ray.origin = world_hit_position + 0.001 * world_normal;
//             ray.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);
//         }
//     }
//     // color = clamp_v(color, 0.0, 100000.0);
//     // color = sqrt(color);
//     let old_color = textureLoad(output_texture, id.xy).rgb;
//     let weight = 1.0 / (f32(path_tracer_uniform.previous_sample_count) + 1.0);
//     var new_color = vec4f(old_color * (1.0 - weight) + color * weight, 1.0);

//     textureStore(accumulation_texture, id.xy, new_color);
//     // textureStore(output_texture, id.xy, new_color);
//     textureStore(output_texture, id.xy, vec4f(color, 1.0));
// }

// ONLY DIFFUSE
// @compute @workgroup_size(16, 16, 1)
// fn path_trace(@builtin(global_invocation_id) id: vec3<u32>) {
//     let pixel_index = id.x + id.y * u32(view.viewport.z);
//     var rng_state = pixel_index + path_tracer_uniform.accumulation_count * 5817321u;

//     let pixel_jitter = pulse::utils::rand_f_pair(&rng_state);
//     var pixel_uv = (vec2<f32>(id.xy) + pixel_jitter) / view.viewport.zw;
//     // Clip position goes from -1 to 1.
//     let pixel_clip_pos = (pixel_uv * 2.0) - 1.0;
//     let ray_target = view.inverse_view_proj * vec4<f32>(pixel_clip_pos.x, -pixel_clip_pos.y, 1.0, 1.0);
//     var ray = Ray(); // Should always be kept in world space.
//     ray.origin = view.world_position;
//     ray.dir = normalize((ray_target.xyz / ray_target.w) - ray.origin);
//     let t_far = 1e30;
//     ray.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);

//     var throughput = vec3f(1.0);
//     var color = vec3f(0.0);
//     let max_depth: u32 = 3u;
//     for (var depth: u32 = 0u; depth < max_depth; depth += 1u){    
//         pulse::utils::trace_ray(&ray);
//         if ray.record.t >= t_far  {
//             // Miss
//             color += throughput * vec3<f32>(0.0, 0.3, 0.3);
//             break;
//         } else {
//             // Hit
//             let instance = instances[ray.record.instance_index];
//             let t_idx = triangle_indices[instance.index_offset + ray.record.triangle_index];
//             let t = triangle_data[instance.triangle_offset + t_idx];
//             let w = 1.0 - (ray.record.u + ray.record.v);
//             let normal = w * t.n_first + ray.record.u * t.n_second + ray.record.v * t.n_third;
//             let world_normal = normalize(pulse::utils::transform_direction(instance.object_world, normal));
//             let world_hit_position = ray.origin + ray.record.t * ray.dir;

//             let material_index = instance.material_index;
//             let material = materials[material_index];

//             color += throughput * material.emissive.xyz * 10.0;
//             throughput *= material.base_color.xyz;

//             let p = max(max(throughput.r, throughput.g), throughput.b);
//             if pulse::utils::rand_f(&rng_state) > p { 
//                 break; 
//             }
//             throughput *= 1.0 / p;

//             let scatter_dir = pulse::utils::sample_hemisphere_rejection(world_normal, &rng_state);
//             // let scatter_dir = sample_cosine_hemisphere_solari(world_normal, &rng_state);
//             // let scatter_dir = scatter_mirror(ray.dir, world_normal);
//             ray.dir = normalize(scatter_dir);
//             ray.origin = world_hit_position + 0.001 * world_normal;
//             ray.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);
//         }
//     }  

//     let old_color = textureLoad(output_texture, id.xy).rgb;
//     let weight = 1.0 / (f32(path_tracer_uniform.accumulation_count) + 1.0);
//     var new_color = vec4f(old_color * (1.0 - weight) + color * weight, 1.0);

//     textureStore(output_texture, id.xy, new_color);
//     // textureStore(output_texture, id.xy, vec4f(1.0, 0.0, 0.0, 1.0));
// }

// ONLY DIFFUSE, TRACE FROM PREPASS TEXTURE
@compute @workgroup_size(16, 16, 1)
fn path_trace(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel_index = id.x + id.y * u32(path_tracer_uniform.width);
    var rng_state = pixel_index * 1235243u + path_tracer_uniform.accumulation_count * 5817321u;
    // var rng_state = pixel_index * 5817321u;

    let pixel_uv = vec2f(id.xy) / vec2f(f32(path_tracer_uniform.width), f32(path_tracer_uniform.height));
    // +0.5 to get to fragment center
    var deferred_texture_coord = vec4f(vec2f(vec2u(pixel_uv * view.viewport.zw)) + 0.5, 0.0, 0.0);
    deferred_texture_coord.z = textureLoad(depth_prepass_texture, vec2<i32>(deferred_texture_coord.xy), 0);

    let deferred_data = textureLoad(deferred_prepass_texture, vec2i(deferred_texture_coord.xy), 0);
    var pbr_input = pbr_input_from_deferred_gbuffer(deferred_texture_coord, deferred_data, view);

    var color_out = vec3f(0.0);

    // Trace first bounce from deferred buffer
    var ray = Ray(); // Should always be kept in world space.
    ray.origin = pbr_input.world_position.xyz + 0.001 * pbr_input.world_normal;
    let scatter_dir = sample_cosine_hemisphere(pbr_input.world_normal, rand_f(&rng_state), rand_f(&rng_state));
    ray.dir = scatter_dir;
    let t_far = 1e30;
    ray.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);

    let direct_light = sample_direct_light(pbr_input.world_position.xyz, pbr_input.world_normal, pbr_input.material.base_color.xyz, &rng_state);
    var color = pbr_input.material.emissive.xyz + direct_light;
    var throughput = pbr_input.material.base_color.xyz;

    let max_depth: u32 = 5u;
    for (var depth: u32 = 0u; depth < max_depth; depth += 1u) {
        trace_ray(&ray);
        if ray.record.t >= t_far  {
            // Miss
            // color += throughput * vec3<f32>(0.03, 0.03, 0.03);
            break;
        } else {
            // Hit
            let instance = instances[ray.record.instance_index];
            let t_idx = triangle_indices[instance.index_offset + ray.record.triangle_index];
            let t = triangle_data[instance.triangle_offset + t_idx];
            let w = 1.0 - (ray.record.u + ray.record.v);
            let normal = w * t.n_first + ray.record.u * t.n_second + ray.record.v * t.n_third;
            let world_normal = normalize(transform_direction(instance.object_world, normal));
            let world_hit_position = ray.origin + ray.record.t * ray.dir;

            let material_index = instance.material_index;
            let material = materials[material_index];

            let direct_light = sample_direct_light(world_hit_position, world_normal, material.base_color.xyz, &rng_state);
            color += throughput * direct_light;
            throughput *= material.base_color.xyz;

            let p = max(max(throughput.r, throughput.g), throughput.b);
            if rand_f(&rng_state) > p { 
                break; 
            }
            throughput *= 1.0 / p;

            let e0 = rand_f(&rng_state);
            let e1 = rand_f(&rng_state);
            let scatter_dir = sample_cosine_hemisphere(world_normal, e0, e1);
            ray.dir = scatter_dir;
            ray.origin = world_hit_position + 0.001 * world_normal;
            ray.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);
        }
    }

    var old_color = vec3f(0.0);
    if path_tracer_uniform.accumulation_count != 0u {
        old_color = textureLoad(output_texture, id.xy).rgb;
    }
    let weight = 1.0 / (f32(path_tracer_uniform.accumulation_count) + 1.0);
    var new_color = vec4f(old_color * (1.0 - weight) + color * weight, 1.0);

    textureStore(output_texture, id.xy, new_color);
}


// Creates a PbrInput from the deferred gbuffer.
fn pbr_input_from_deferred_gbuffer(frag_coord: vec4<f32>, gbuffer: vec4<u32>, view: View) -> bevy_pbr::pbr_types::PbrInput {
    var pbr = pbr_types::pbr_input_new();

    let flags = pbr_deferred_types::unpack_flags(gbuffer.a);
    let deferred_flags = pbr_deferred_types::mesh_material_flags_from_deferred_flags(flags);
    pbr.flags = deferred_flags.x;
    pbr.material.flags = deferred_flags.y;

    let base_rough = pbr_deferred_types::unpack_unorm4x8_(gbuffer.r);
    pbr.material.perceptual_roughness = base_rough.a;
    let emissive = rgb9e5::rgb9e5_to_vec3_(gbuffer.g);
    if ((pbr.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u) {
        pbr.material.base_color = vec4(emissive, 1.0);
        pbr.material.emissive = vec4(vec3(0.0), 1.0);
    } else {
        pbr.material.base_color = vec4(pow(base_rough.rgb, vec3(2.2)), 1.0);
        pbr.material.emissive = vec4(emissive, 1.0);
    }

    let props = pbr_deferred_types::unpack_unorm4x8_(gbuffer.b);
    pbr.material.reflectance = props.r;

    pbr.material.metallic = props.g;
    pbr.diffuse_occlusion = vec3(props.b);
    let octahedral_normal = pbr_deferred_types::unpack_24bit_normal(gbuffer.a);
    let N = utils::octahedral_decode(octahedral_normal);

    let world_position = vec4(position_ndc_to_world(frag_coord_to_ndc(frag_coord, view), view), 1.0);
    let is_orthographic = view.projection[3].w == 1.0;
    let V = calculate_view(world_position, is_orthographic, view);

    pbr.frag_coord = frag_coord;
    pbr.world_normal = N;
    pbr.world_position = world_position;
    pbr.N = N;
    pbr.V = V;
    pbr.is_orthographic = is_orthographic;

    return pbr;
}

/// Convert a ndc space position to world space
fn position_ndc_to_world(ndc_pos: vec3<f32>, view: View) -> vec3<f32> {
    let world_pos = view.inverse_view_proj * vec4(ndc_pos, 1.0);
    return world_pos.xyz / world_pos.w;
}

/// Convert frag coord to ndc
fn frag_coord_to_ndc(frag_coord: vec4<f32>, view: View) -> vec3<f32> {
    return vec3(uv_to_ndc(frag_coord_to_uv(frag_coord.xy, view)), frag_coord.z);
}

/// returns the (0.0, 0.0) .. (1.0, 1.0) position within the viewport for the current render target
/// [0 .. render target viewport size] eg. [(0.0, 0.0) .. (1280.0, 720.0)] to [(0.0, 0.0) .. (1.0, 1.0)]
fn frag_coord_to_uv(frag_coord: vec2<f32>, view: View) -> vec2<f32> {
    return (frag_coord - view.viewport.xy) / view.viewport.zw;
}

/// Convert uv [0.0 .. 1.0] coordinate to ndc space xy [-1.0 .. 1.0]
fn uv_to_ndc(uv: vec2<f32>) -> vec2<f32> {
    return uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
}


// NOTE: Correctly calculates the view vector depending on whether
// the projection is orthographic or perspective.
fn calculate_view(
    world_position: vec4<f32>,
    is_orthographic: bool,
    view: View,
) -> vec3<f32> {
    var V: vec3<f32>;
    if is_orthographic {
        // Orthographic view vector
        V = normalize(vec3<f32>(view.view_proj[0].z, view.view_proj[1].z, view.view_proj[2].z));
    } else {
        // Only valid for a perpective projection
        V = normalize(view.world_position.xyz - world_position.xyz);
    }
    return V;
}
