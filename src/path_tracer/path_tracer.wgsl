#import bevy_pbr::{
    prepass_utils::prepass_depth,
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
    pbr_functions,
    pbr_deferred_functions::pbr_input_from_deferred_gbuffer,
    pbr_deferred_types::unpack_unorm3x4_plus_unorm_20_,
    mesh_view_bindings::{depth_prepass_texture, deferred_prepass_texture, view},
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

@group(2) @binding(0) var output_texture: texture_storage_2d<rgba32float, read_write>;
@group(2) @binding(1) var<uniform> path_tracer_uniform: PathTracerUniform;

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
    var rng_state = pixel_index * 5817321u;

    let pixel_uv = vec2f(id.xy) / vec2f(f32(path_tracer_uniform.width), f32(path_tracer_uniform.height));
    // + 0.5 to get to fragment center
    var deferred_texture_coord = vec4f(vec2f(vec2u(pixel_uv * view.viewport.zw)) + 0.5, 0.0, 0.0);
    // var frag_coord = vec4f(fragment_position, 0.0, 0.0);
    deferred_texture_coord.z = prepass_depth(deferred_texture_coord, 0u);
    let deferred_data = textureLoad(deferred_prepass_texture, vec2i(deferred_texture_coord.xy), 0);
    var pbr_input = pbr_input_from_deferred_gbuffer(deferred_texture_coord, deferred_data);

    var color_out = vec3f(0.0);

    // Trace first bounce from deferred buffer
    var ray = Ray(); // Should always be kept in world space.
    // let scatter_dir = sample_cosine_hemisphere(pbr_input.world_normal, rand_f(&rng_state), rand_f(&rng_state));
    // let scatter_dir = sample_cosine_hemisphere(vec3f(1.0, 0.0, 0.0), 0.1, 0.7);



    let e0 = 0.1;
    let e1 = 0.3;
    let cos_theta = 2.0 * e0 - 1.0;
    let phi = 2.0 * 3.14 * e1;
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let sin_phi = sin(phi);
    let cos_phi = cos(phi);
    let unit_sphere_direction = normalize(vec3(sin_theta * cos_phi, cos_theta, sin_theta * sin_phi));
    let scatter_dir = normalize(pbr_input.world_normal + unit_sphere_direction);
    
    ray.origin = pbr_input.world_position.xyz + 0.001 * pbr_input.world_normal;
    ray.dir = scatter_dir;
    let t_far = 1e30;
    ray.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);

    // // let direct_light = sample_direct_light(pbr_input.world_position.xyz, pbr_input.world_normal, pbr_input.material.base_color.xyz, &rng_state);
    // let direct_light = vec3f(0.2);
    // var color = pbr_input.material.emissive.xyz + direct_light;
    // var throughput = pbr_input.material.base_color.xyz;

    // let max_depth: u32 = 0u;
    // for (var depth: u32 = 0u; depth < max_depth; depth += 1u) {
    //     trace_ray(&ray);
    //     if ray.record.t >= t_far  {
    //         // Miss
    //         // color += throughput * vec3<f32>(0.03, 0.03, 0.03);
    //         break;
    //     } else {
    //         // Hit
    //         let instance = instances[ray.record.instance_index];
    //         let t_idx = triangle_indices[instance.index_offset + ray.record.triangle_index];
    //         let t = triangle_data[instance.triangle_offset + t_idx];
    //         let w = 1.0 - (ray.record.u + ray.record.v);
    //         let normal = w * t.n_first + ray.record.u * t.n_second + ray.record.v * t.n_third;
    //         let world_normal = normalize(transform_direction(instance.object_world, normal));
    //         let world_hit_position = ray.origin + ray.record.t * ray.dir;

    //         let material_index = instance.material_index;
    //         let material = materials[material_index];

    //         // let direct_light = pulse::utils::sample_direct_light(world_hit_position, world_normal, material.base_color.xyz, &rng_state);
    //         let direct_light = vec3f(0.2);
    //         color += throughput * direct_light;
    //         throughput *= material.base_color.xyz;

    //         let p = max(max(throughput.r, throughput.g), throughput.b);
    //         if rand_f(&rng_state) > p { 
    //             break; 
    //         }
    //         throughput *= 1.0 / p;

    //         let e0 = rand_f(&rng_state);
    //         let e1 = rand_f(&rng_state);
    //         let scatter_dir = sample_cosine_hemisphere(world_normal, e0, e1);
    //         // let scatter_dir = pulse::utils::sample_hemisphere_rejection(world_normal, &rng_state);
    //         ray.dir = scatter_dir;
    //         ray.origin = world_hit_position + 0.001 * world_normal;
    //         ray.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);
    //     }
    // }

    // let old_color = textureLoad(output_texture, id.xy).rgb;
    // let weight = 1.0 / (f32(path_tracer_uniform.accumulation_count) + 1.0);
    // var new_color = vec4f(old_color * (1.0 - weight) + color * weight, 1.0);

    // textureStore(output_texture, id.xy, new_color);
    textureStore(output_texture, id.xy, vec4f(1.0, 0.0, 0.0, 1.0));

}
