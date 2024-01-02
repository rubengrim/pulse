#import pulse::scene::{
    types::{
        Primitive,
        TriangleData,
        BLASNode,
        TLASNode,
        MeshInstance,
        SceneUniform,
        Material,
        Ray,
        RayHitRecord,
    }, 
    bindings::{
        scene_uniform,
        primitives,
        triangle_data,
        triangle_indices,
        blas_nodes,
        tlas_nodes,
        instance_indices,
        instances,
        materials,
    }
}
#import bevy_render::view::View

const PI: f32 = 3.141592653589793;

struct PathTracerUniform {
    previous_sample_count: u32,
}

@group(1) @binding(0) var<uniform> view: View;
@group(1) @binding(1) var<uniform> path_tracer_uniform: PathTracerUniform;
@group(1) @binding(2) var output_texture: texture_storage_2d<rgba32float, read_write>;
@group(1) @binding(3) var accumulation_texture: texture_storage_2d<rgba32float, read_write>;


@compute @workgroup_size(16, 16, 1)
fn path_trace(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel_index = id.x + id.y * u32(view.viewport.z);
    var rng_state = pixel_index + path_tracer_uniform.previous_sample_count * 5817321u;

    let pixel_jitter = rand_f_pair(&rng_state);
    var pixel_uv = (vec2<f32>(id.xy) + pixel_jitter) / view.viewport.zw;
    // Clip position goes from -1 to 1.
    let pixel_clip_pos = (pixel_uv * 2.0) - 1.0;
    let ray_target = view.inverse_view_proj * vec4<f32>(pixel_clip_pos.x, -pixel_clip_pos.y, 1.0, 1.0);
    var ray = Ray(); // Should always be kept in world space.
    ray.origin = view.world_position;
    ray.dir = normalize((ray_target.xyz / ray_target.w) - ray.origin);
    let t_far = 1e30;
    ray.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);

    var throughput = vec3f(1.0);
    var color_out = vec3f(0.0);
    let max_depth: u32 = 3u;
    for (var depth: u32 = 0u; depth < max_depth; depth += 1u){    
        trace_ray_tlas(&ray);
        if ray.record.t >= t_far  {
            // Miss
            color_out += throughput * vec3<f32>(0.0, 0.3, 0.3);
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

            color_out += throughput * material.emissive.xyz * 10.0;
            throughput *= material.base_color.xyz;

            let p = max(max(throughput.r, throughput.g), throughput.b);
            if rand_f(&rng_state) > p { 
                break; 
            }
            throughput *= 1.0 / p;

            let scatter_dir = sample_hemisphere_rejection(world_normal, &rng_state);
            // let scatter_dir = sample_cosine_hemisphere_solari(world_normal, &rng_state);
            // let scatter_dir = scatter_mirror(ray.dir, world_normal);
            ray.dir = normalize(scatter_dir);
            ray.origin = world_hit_position + 0.001 * world_normal;
            ray.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);
        }
    }  
    let old_color = textureLoad(accumulation_texture, id.xy).rgb;
    // let new_color = vec4f((color_out + f32(path_tracer_uniform.previous_sample_count) * old_color) / (f32(path_tracer_uniform.sample_accumulation_count) + 1.0), 1.0);
    let weight = 1.0 / (f32(path_tracer_uniform.previous_sample_count) + 1.0);
    let new_color = vec4f(old_color * (1.0 - weight) + color_out * weight, 1.0);

    textureStore(accumulation_texture, id.xy, new_color);
    textureStore(output_texture, id.xy, new_color);
}

fn trace_ray_tlas(ray: ptr<function, Ray>) {
    traverse_tlas(ray);
}

fn trace_ray(ray: ptr<function, Ray>)  {
    for (var i = 0u; i < scene_uniform.instance_count; i += 1u) {
        traverse_blas(ray, i);
    }

}

fn trace_ray_brute(ray: ptr<function, Ray>)  {
    for (var i = 0u; i < scene_uniform.instance_count; i += 1u) {

    }
}

fn get_blas_node(index: u32, instance_index: u32) -> BLASNode {
    let instance = instances[instance_index];
    return blas_nodes[index + instance.node_offset];

    // return blas_nodes[index];
}

fn get_primitive(index: u32, instance_index: u32) -> Primitive {
    let instance = instances[instance_index];
    let triangle_index = triangle_indices[index + instance.index_offset];
    return primitives[triangle_index + instance.triangle_offset];

    // let triangle_index = triangle_indices[index];
    // return primitives[triangle_index];
}

fn get_triangle_data(index: u32, instance_index: u32) -> TriangleData {
    let instance = instances[instance_index];
    let triangle_index = triangle_indices[index + instance.index_offset];
    return triangle_data[triangle_index + instance.triangle_offset];

    // let triangle_index = triangle_indices[index];
    // return triangle_data[triangle_index];
}

fn traverse_tlas(ray: ptr<function, Ray>) {
    // Abort on empty/invalid root node.
    if tlas_nodes[0].a_or_first_instance == 0u && tlas_nodes[0].instance_count == 0u {
        return;
    }

    var node_index = 0u;
    var stack: array<u32, 32>;
    var stack_ptr = 0;
    var iteration = 0;
    let max_iterations = 100000;
    while iteration < max_iterations {
        iteration += 1;
        let node = tlas_nodes[node_index];


        if node.instance_count > 0u { // Is leaf node.
            for (var i: u32 = 0u; i < node.instance_count; i += 1u) {
                let instance_index = instance_indices[node.a_or_first_instance + i];
                traverse_blas(ray, instance_index);
            }
            if stack_ptr == 0 {
                break;
            } else {
                stack_ptr -= 1;
                node_index = stack[stack_ptr];
            }
            continue;
        }

        // Current node is an interior node, so visit child nodes in order.
        var child_a_index = node.a_or_first_instance;
        var child_b_index = child_a_index + 1u;
        let child_a = tlas_nodes[child_a_index];
        let child_b = tlas_nodes[child_b_index];
        var dist_a = ray_aabb_intersect(ray, child_a.aabb_min, child_a.aabb_max);
        var dist_b = ray_aabb_intersect(ray, child_b.aabb_min, child_b.aabb_max);
        if dist_a > dist_b {
            let d = dist_a;
            dist_a = dist_b;
            dist_b = d;
            let c = child_a_index;
            child_a_index = child_b_index;
            child_b_index = c;
        }
        if dist_a == 1e30f {
            // Missed both child nodes.
            if stack_ptr == 0 {
                break;
            } else  {
                stack_ptr -= 1;
                node_index = stack[stack_ptr];
            }
        } else {
            // Use near node next and push the far node if it's intersected by the ray.
            node_index = child_a_index;
            if dist_b != 1e30f {
                stack[stack_ptr] = child_b_index;
                stack_ptr += 1;
            }
        }
    }
}

// Takes world space ray.
fn traverse_blas(ray: ptr<function, Ray>, instance_index: u32) {
    // Transform ray to object/blas space.
    let instance = instances[instance_index];
    var ray_object = Ray();
    ray_object.origin = transform_position(instance.world_object, (*ray).origin);
    ray_object.dir = normalize(transform_direction(instance.world_object, (*ray).dir));
    let t_far = 1e30;
    ray_object.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);

    var node_index = 0u;
    var stack: array<u32, 32>;
    var stack_ptr = 0;
    var iteration = 0;
    let max_iterations = 100000;
    while iteration < max_iterations {
        iteration += 1;
        let node = get_blas_node(node_index, instance_index);
        if node.tri_count > 0u {
            for (var i: u32 = 0u; i < node.tri_count; i += 1u) {
                ray_triangle_intersect(&ray_object, node.a_or_first_tri + i, instance_index);
            }
            if stack_ptr == 0 {
                break;
            } else {
                stack_ptr -= 1;
                node_index = stack[stack_ptr];
            }
            continue;
        }

        var child_a_index = node.a_or_first_tri;
        var child_b_index = child_a_index + 1u;
        let child_a = get_blas_node(child_a_index, instance_index);
        let child_b = get_blas_node(child_b_index, instance_index);
        var dist_a = ray_aabb_intersect(&ray_object, child_a.aabb_min, child_a.aabb_max);
        var dist_b = ray_aabb_intersect(&ray_object, child_b.aabb_min, child_b.aabb_max);
        if dist_a > dist_b {
            let d = dist_a;
            dist_a = dist_b;
            dist_b = d;
            let c = child_a_index;
            child_a_index = child_b_index;
            child_b_index = c;
        }
        if dist_a == 1e30f {
            if stack_ptr == 0 {
                break;
            } else  {
                stack_ptr -= 1;
                node_index = stack[stack_ptr];
            }
        } else {
            node_index = child_a_index;
            if dist_b != 1e30f {
                stack[stack_ptr] = child_b_index;
                stack_ptr += 1;
            }
        }
    }

    let hit_position_object = ray_object.origin + ray_object.record.t * ray_object.dir;
    let hit_position_world = transform_position(instance.object_world, hit_position_object);
    let new_t_world = length(hit_position_world - (*ray).origin);
    if new_t_world < (*ray).record.t {
        (*ray).record = ray_object.record;
        (*ray).record.t = new_t_world;
    }
}

fn ray_aabb_intersect(ray: ptr<function, Ray>, aabb_min: vec3<f32>, aabb_max: vec3<f32>) -> f32 {
    let t_x_1 = (aabb_min.x - (*ray).origin.x) / (*ray).dir.x;
    let t_x_2 = (aabb_max.x - (*ray).origin.x) / (*ray).dir.x;
    var t_min = min(t_x_1, t_x_2); 
    var t_max = max(t_x_1, t_x_2); 

    let t_y_1 = (aabb_min.y - (*ray).origin.y) / (*ray).dir.y;
    let t_y_2 = (aabb_max.y - (*ray).origin.y) / (*ray).dir.y;
    t_min = max(t_min, min(t_y_1, t_y_2)); 
    t_max = min(t_max, max(t_y_1, t_y_2)); 

    let t_z_1 = (aabb_min.z - (*ray).origin.z) / (*ray).dir.z;
    let t_z_2 = (aabb_max.z - (*ray).origin.z) / (*ray).dir.z;
    t_min = max(t_min, min(t_z_1, t_z_2)); 
    t_max = min(t_max, max(t_z_1, t_z_2)); 

    if (t_max >= t_min && t_min < (*ray).record.t && t_max > 0.0) {
        return t_min;
    } else {
        return 1e30f;
    }
}

// fn ray_sphere_intersect(ray: ptr<function, Ray>, s0: vec3<f32>, sr: f32){
//     // let t = 1.0;
//     // (*ray).record.t = t;
//     // let hit_pos = (*ray).origin + t * (*ray).dir;
//     // (*ray).record.normal = normalize(hit_pos - s0);

//     let a = dot((*ray).dir, (*ray).dir);
//     let s0_r0 = (*ray).origin - s0;
//     let b = 2.0 * dot((*ray).dir, s0_r0);
//     let c = dot(s0_r0, s0_r0) - (sr * sr);
//     if (b*b - 4.0*a*c >= 0.0) {
//         let t = (-b - sqrt((b*b) - 4.0*a*c))/(2.0*a);
//         if (t > 0.001 && (*ray).record.t > t) {
//             (*ray).record.t = t;
//             let hit_pos = (*ray).origin + t * (*ray).dir;
//             (*ray).record.normal = normalize(hit_pos - s0);
//         }
//     }
// }

// Moeller-Trumbore ray/triangle intersection algorithm
// Updates ray hit record if new t is smaller
fn ray_triangle_intersect(ray: ptr<function, Ray>, triangle_index: u32, instance_index: u32) {
    let prim = get_primitive(triangle_index, instance_index);
    let edge_1 = prim.p_second- prim.p_first;
    let edge_2 = prim.p_third- prim.p_first;
    let h = cross((*ray).dir, edge_2);
    let a = dot(edge_1, h);
    // if a > -0.0001 && a < 0.0001 { // Ray parallel to triangle
    if abs(a) < 0.0001 { // Ray parallel to triangle
        return;
    }
    let f = 1.0 / a;
    let s = (*ray).origin - prim.p_first;
    let u = f * dot(s, h);
    if u < 0.0 || u > 1.0 {
        return;
    }
    let q = cross(s, edge_1);
    let v = f * dot((*ray).dir, q);
    if v < 0.0 || u + v > 1.0 {
        return;
    }
    let t = f * dot(edge_2, q);
    if t > 0.001 && t < (*ray).record.t  {
        let tri_data = get_triangle_data(triangle_index, instance_index);
        (*ray).record.t = t;
        (*ray).record.instance_index = instance_index;
        (*ray).record.triangle_index = triangle_index;
        (*ray).record.u = u;
        (*ray).record.v = v;
    }
}

fn transform_position(m: mat4x4f, p: vec3f) -> vec3f {
    let h = m * vec4f(p, 1.0);
    return h.xyz / h.w;
}

fn transform_direction(m: mat4x4f, p: vec3f) -> vec3f {
    let h = m * vec4f(p, 0.0);
    return h.xyz;
}

fn transform_normal(m: mat4x4f, p: vec3f) -> vec3f {
    let h = transpose(m) * vec4f(p, 0.0);
    return h.xyz;
}

fn rand_u(state: ptr<function, u32>) -> u32 {
    // PCG hash
    *state = *state * 747796405u + 2891336453u;
    let word = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn rand_f(state: ptr<function, u32>) -> f32 {
    // PCG hash
    *state = *state * 747796405u + 2891336453u;
    let word = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    return f32((word >> 22u) ^ word) * bitcast<f32>(0x2f800004u);
}

fn rand_f_pair(state: ptr<function, u32>) -> vec2<f32> {
    return vec2(rand_f(state), rand_f(state));
}

fn rand_range_u(n: u32, state: ptr<function, u32>) -> u32 {
    return rand_u(state) % n;
}

fn sample_cosine_hemisphere_solari(normal: vec3<f32>, state: ptr<function, u32>) -> vec3<f32> {
    let cos_theta = 2.0 * rand_f(state) - 1.0;
    let phi = 2.0 * PI * rand_f(state);
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let sin_phi = sin(phi);
    let cos_phi = cos(phi);
    let unit_sphere_direction = normalize(vec3(sin_theta * cos_phi, cos_theta, sin_theta * sin_phi));
    return normal + unit_sphere_direction;
}


fn scatter_mirror(in: vec3f, normal: vec3f) -> vec3f {
    return in - 2.0 * dot(in, normal) * normal;
}

fn sample_hemisphere_rejection(normal: vec3f, state: ptr<function, u32>) -> vec3f {
    loop {
        let x = rand_f(state) * 2.0 - 1.0;
        let y = rand_f(state) * 2.0 - 1.0;
        let z = rand_f(state) * 2.0 - 1.0;

        var candidate = vec3f(x, y, z);
        if length(candidate) <= 1.0 && dot(candidate, normal) > 0.0 {
            candidate = normalize(candidate);
            return candidate;
        }
    }
    return normal;
}
