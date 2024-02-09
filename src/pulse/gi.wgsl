#import bevy_pbr::{
    prepass_utils,
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
    pbr_functions,
    pbr_deferred_functions::pbr_input_from_deferred_gbuffer,
    pbr_deferred_types::unpack_unorm3x4_plus_unorm_20_,
    mesh_view_bindings::{depth_prepass_texture, deferred_prepass_texture, view},
}
#import bevy_render::view::View
#import pulse::{
    utilities::{
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

// TODO: Use constant value for ray origin offset

@group(2) @binding(0) var pulse_output_texture: texture_storage_2d<rgba32float, read_write>;

// @compute @workgroup_size(16, 16, 1)
// fn gi(@builtin(global_invocation_id) id: vec3<u32>) { 
//     let pixel_index = id.x + id.y * u32(view.viewport.z);
//     var rng_state = pixel_index * 5817321u;

//     var color_out = vec3f(0.0);
//     let spp = 2u;
//     let spp_inv = 1.0 / f32(spp);
//     for (var sample: u32 = 0u; sample < spp; sample += 1u) {
//         // let pixel_jitter = rand_f_pair(&rng_state);
//         let pixel_jitter = vec2f(0.0);
//         var pixel_uv = (vec2<f32>(id.xy) + pixel_jitter) / view.viewport.zw;

//         // Clip position goes from -1 to 1.
//         let pixel_clip_pos = (pixel_uv * 2.0) - 1.0;
//         let ray_target = view.inverse_view_proj * vec4<f32>(pixel_clip_pos.x, -pixel_clip_pos.y, 1.0, 1.0);
//         var ray = Ray(); // Should always be kept in world space.
//         ray.origin = view.world_position;
//         ray.dir = normalize((ray_target.xyz / ray_target.w) - ray.origin);
//         let t_far = 1e30;
//         ray.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);

//         var throughput = vec3f(1.0);
//         var color = vec3f(0.0);

//         let max_depth: u32 = 3u;
//         for (var depth: u32 = 0u; depth < max_depth; depth += 1u) {
//             trace_ray(&ray);
//             if ray.record.t >= t_far  {
//                 // Miss
//                 // color += throughput * vec3<f32>(0.03, 0.03, 0.03);
//                 break;
//             } else {
//                 // Hit
//                 let instance = instances[ray.record.instance_index];
//                 let t_idx = triangle_indices[instance.index_offset + ray.record.triangle_index];
//                 let t = triangle_data[instance.triangle_offset + t_idx];
//                 let w = 1.0 - (ray.record.u + ray.record.v);
//                 let normal = w * t.n_first + ray.record.u * t.n_second + ray.record.v * t.n_third;
//                 let world_normal = normalize(transform_direction(instance.object_world, normal));
//                 let world_hit_position = ray.origin + ray.record.t * ray.dir;

//                 let material_index = instance.material_index;
//                 let material = materials[material_index];

//                 if depth == 0u {
//                     color += throughput * material.emissive.xyz;
//                 }

//                 let direct_light = sample_direct_light(world_hit_position, world_normal, material.base_color.xyz, &rng_state);
//                 color += throughput * direct_light;
//                 throughput *= material.base_color.xyz;



//                 // color += throughput * material.emissive.xyz;
//                 // throughput *= material.base_color.xyz;

//                 let p = max(max(throughput.r, throughput.g), throughput.b);
//                 if rand_f(&rng_state) > p { 
//                     break; 
//                 }
//                 throughput *= 1.0 / p;

//                 let e0 = rand_f(&rng_state);
//                 let e1 = rand_f(&rng_state);
//                 let scatter_dir = sample_cosine_hemisphere(world_normal, e0, e1);
//                 // let scatter_dir = pulse::utilities::sample_hemisphere_rejection(world_normal, &rng_state);
//                 ray.dir = scatter_dir;
//                 ray.origin = world_hit_position + 0.001 * world_normal;
//                 ray.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);
//             }
//         }  

//         color_out += color * spp_inv;
//     }

//     textureStore(pulse_output_texture, id.xy, vec4f(color_out, 1.0));
// }

// Trace from deferred texture
@compute @workgroup_size(16, 16, 1)
fn gi(@builtin(global_invocation_id) id: vec3<u32>) { 
    let pixel_index = id.x + id.y * u32(view.viewport.z);
    var rng_state = pixel_index * 5817321u;

    // Fragment position pixel center is offset 0.5 from integer number.
    // https://www.w3.org/TR/WGSL/#position-builtin-value
    let fragment_position = vec2f(id.xy) + 0.5;
    var frag_coord = vec4f(fragment_position, 0.0, 0.0);
    frag_coord.z = prepass_utils::prepass_depth(frag_coord, 0u);
    let deferred_data = textureLoad(deferred_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    var pbr_input = pbr_input_from_deferred_gbuffer(frag_coord, deferred_data);

    var color_out = vec3f(0.0);
    let spp = 2u;
    let spp_inv = 1.0 / f32(spp);
    for (var sample: u32 = 0u; sample < spp; sample += 1u) {
        // Trace first bounce from deferred buffer
        var ray = Ray(); // Should always be kept in world space.
        let scatter_dir = sample_cosine_hemisphere(pbr_input.world_normal, rand_f(&rng_state), rand_f(&rng_state));
        ray.origin = pbr_input.world_position.xyz + 0.001 * pbr_input.world_normal;
        ray.dir = scatter_dir;
        let t_far = 1e30;
        ray.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);

        let direct_light = sample_direct_light(pbr_input.world_position.xyz, pbr_input.world_normal, pbr_input.material.base_color.xyz, &rng_state);
        var color = pbr_input.material.emissive.xyz + direct_light;
        var throughput = pbr_input.material.base_color.xyz;

        // No NNE
        // var color = pbr_input.material.emissive.xyz;
        // var throughput = pbr_input.material.base_color.xyz;

        let max_depth: u32 = 1u;
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

                // No NNE
                // color += throughput * material.emissive.xyz;
                // throughput *= material.base_color.xyz;

                let p = max(max(throughput.r, throughput.g), throughput.b);
                if rand_f(&rng_state) > p { 
                    break; 
                }
                throughput *= 1.0 / p;

                let e0 = rand_f(&rng_state);
                let e1 = rand_f(&rng_state);
                let scatter_dir = sample_cosine_hemisphere(world_normal, e0, e1);
                // let scatter_dir = pulse::utils::sample_hemisphere_rejection(world_normal, &rng_state);
                ray.dir = scatter_dir;
                ray.origin = world_hit_position + 0.001 * world_normal;
                ray.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);
            }
        }  
        // color = max(color, vec3f(0.0));
        color_out += color * spp_inv;
    }

    textureStore(pulse_output_texture, id.xy, vec4f(color_out, 1.0));
}

// `p0`/`n0`/`base_color` are position/normal/color of point from where to sample
fn sample_direct_light(p0: vec3f, n0: vec3f, base_color: vec3f, rng_state: ptr<function, u32>) -> vec3f {
    // let light_index = sample_light_emission_strength_cdf(rand_f(rng_state));
    let light_index = rand_range_u(scene_uniform.light_count, rng_state);
    let light_data_index = light_indices[light_index];
    let mesh_instance = instances[light_data_index.mesh_instance_index];

    // Uniformly sample a point on the surface of the chosen light.
    let e0 = rand_f(rng_state);
    let e1 = rand_f(rng_state);
    let e2 = rand_f(rng_state);
    let primitive_index = sample_light_triangle_area_cdf(e0, light_data_index.cdf_offset, mesh_instance.triangle_count);
    let primitive = primitives[mesh_instance.triangle_offset + primitive_index];

    let pl_obj = sample_triangle_uniformly(e1, e2, primitive.p_first, primitive.p_second, primitive.p_third);
    let pl = pulse::utilities::transform_position(mesh_instance.object_world, pl_obj);

    let to_light = pl - p0;
    var shadow_ray = Ray();
    shadow_ray.origin = p0 + 0.001 * n0;
    shadow_ray.dir = normalize(to_light);
    shadow_ray.record = RayHitRecord(1e30, 0u, 0u, 0.0, 0.0);
    trace_ray(&shadow_ray);

    if shadow_ray.record.t < length(pl - shadow_ray.origin) - 0.005 {
        return vec3f(0.0);
    }
    
    // if trace_shadow_ray(&shadow_ray, length(to_light) - 0.0001) {
    //     // Light source is occluded; no direct light contribution
    //     return vec3f(0.0);
    // }

    // Calculate triangle normal. Could try to interpolate normals but this should be good enough.
    let side_a = primitive.p_second - primitive.p_first;
    let side_b = primitive.p_third - primitive.p_first;
    var nl = normalize(cross(side_a, side_b));
    if dot(nl, -shadow_ray.dir) < 0.0 {
        nl = -nl;
    }

    var light_pdf: f32;
    // if light_index == 0u {
    //     light_pdf = light_emission_strength_cdf[light_index];
    // } else {
    //     light_pdf = light_emission_strength_cdf[light_index] - light_emission_strength_cdf[light_index - 1u];
    // }
    light_pdf = 1.0 / f32(scene_uniform.light_count);
    var triangle_pdf = 1.0 / light_mesh_areas[light_index];
    let pdf = light_pdf * triangle_pdf;

    // Evaluate direct light contribution
    // https://www.youtube.com/watch?v=FU1dbi827LY at 4:24
    let cos_theta_receiver = dot(shadow_ray.dir, n0);
    let cos_theta_emitter = dot(-shadow_ray.dir, nl);
    let brdf = base_color * pulse::utilities::INV_PI;
    let light_material = materials[mesh_instance.material_index];
    let direct_light = brdf * light_material.emissive.xyz * cos_theta_receiver * cos_theta_emitter / pdf / distance_sq(n0, nl);
    return max(direct_light, vec3f(0.0));
}

// Find the index of the largest value <= `e` between in `light_emission_strength_cdf`.
// Binary search
fn sample_light_emission_strength_cdf(e: f32) -> u32 {
    var l: i32 = 0;
    var r: i32 = l + i32(scene_uniform.light_count) - 1;
    while l <= r {
        let mid = l + (r - l);

        if light_emission_strength_cdf[mid] <= e {
            l = mid + 1;
        } else {
            r = mid - 1;
        }
    }

    return min(u32(l), scene_uniform.light_count - 1u);
}

// Find the index of the largest value <= `e` between `cdf_offset`and `cdf_offset` + `count` in `light_cdfs`.
// Binary search
fn sample_light_triangle_area_cdf(e: f32, cdf_offset: u32, count: u32) -> u32 {
    var l: i32 = i32(cdf_offset);
    var r: i32 = l + i32(count) - 1;
    while l <= r {
        let mid = l + (r - l);

        if light_triangle_area_cdfs[mid] <= e {
            l = mid + 1;
        } else {
            r = mid - 1;
        }
    }

    return min(u32(l), cdf_offset + count - 1u) - cdf_offset;
}

// Parallelogram method
// https://extremelearning.com.au/evenly-distributing-points-in-a-triangle/
fn sample_triangle_uniformly(e0: f32, e1: f32, p0: vec3f, p1: vec3f, p2: vec3f) -> vec3f {
    let a = p1 - p0;
    let b = p2 - p0;
    if e0 + e1 < 1.0 {
        return p0 + (e0 * a) + (e1 * b);
    } else {
        return p0 + ((1.0 - e0) * a) + ((1.0 - e1) * b);
    }
}
