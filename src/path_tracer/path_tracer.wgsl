// #import pulse::scene_bindings:
#import bevy_render::view::View

struct Primitive {
    p0: vec3<f32>,
    p1: vec3<f32>,
    p2: vec3<f32>,
}

struct TriangleData {
    n0: vec3<f32>,
    n1: vec3<f32>,
    n2: vec3<f32>,
    uv0: vec2<f32>,
    uv1: vec2<f32>,
    uv2: vec2<f32>,
}

struct BLASNode {
    aabb_min: vec3<f32>,
    a_or_first_tri: u32,
    aabb_max: vec3<f32>,
    tri_count: u32,
}

struct TLASNode {
    aabb_min: vec3<f32>,
    a_or_first_instance: u32,
    aabb_max: vec3<f32>,
    instance_count: u32,
}

struct MeshInstance {
    object_world: mat4x4f,
    world_object: mat4x4f,
    triangle_offset: u32,
    // triangle_count: u32,
    index_offset: u32,
    // index_count: u32,
    node_offset: u32,
    // node_count: u32,
}

struct SceneUniform {
    instance_count: u32,
}

@group(0) @binding(0) var<uniform> scene_uniform: SceneUniform;
@group(0) @binding(1) var<storage> primitives: array<Primitive>;
@group(0) @binding(2) var<storage> triangle_data: array<TriangleData>;
@group(0) @binding(3) var<storage> triangle_indices: array<u32>;
@group(0) @binding(4) var<storage> blas_nodes: array<BLASNode>;
@group(0) @binding(5) var<storage> tlas_nodes: array<TLASNode>;
@group(0) @binding(6) var<storage> instance_indices: array<u32>;
@group(0) @binding(7) var<storage> instances: array<MeshInstance>;

@group(1) @binding(0) var<uniform> view: View;
@group(1) @binding(1) var output_texture: texture_storage_2d<rgba16float, read_write>;

struct Ray {
    origin: vec3f,
    dir: vec3f,
    record: RayHitRecord,
}

struct RayHitRecord {
    t: f32,
    instance_index: u32,
    triangle_index: u32,
    // Barycentric coordinates of hit position
    u: f32,
    v: f32,
}

@compute @workgroup_size(16, 16, 1)
fn path_trace(@builtin(global_invocation_id) id: vec3<u32>) {
    var pixel_uv = (vec2<f32>(id.xy) + 0.5) / view.viewport.zw;
    // Clip position goes from -1 to 1.
    let pixel_clip_pos = (pixel_uv * 2.0) - 1.0;
    let ray_target = view.inverse_view_proj * vec4<f32>(pixel_clip_pos.x, -pixel_clip_pos.y, 1.0, 1.0);
    var ray = Ray();
    ray.origin = view.world_position;
    ray.dir = normalize((ray_target.xyz / ray_target.w) - ray.origin);
    let t_far = 1e30;
    ray.record = RayHitRecord(t_far, 0u, 0u, 0.0, 0.0);

    var color_out: vec4<f32>;
    trace_ray_tlas(&ray);
    // trace_ray(&ray);
    if ray.record.t < 0.1 || ray.record.t >= t_far  {
        // Miss
        color_out = vec4<f32>(0.0, 0.3, 0.3, 1.0);
    } else {
        // Hit
        let instance = instances[ray.record.instance_index];
        let t_idx = triangle_indices[instance.index_offset + ray.record.triangle_index];
        let t = triangle_data[instance.triangle_offset + t_idx];
        let w = 1.0 - (ray.record.u + ray.record.v);
        let normal_interpolated = ray.record.u * t.n0 + ray.record.v * t.n1 + w * t.n2;
        let normal_world = normalize(transform_direction(instance.object_world, normal_interpolated));
        let hit_position_world = transform_position(instance.object_world, ray.origin + ray.record.t * ray.dir);

        let light_position = vec3<f32>(2.0, 5.0, 1.0);
        let light_strength = 15.0;
        // let to_light = light_position - hit_position_world;
        let to_light = vec3f(0.0, 1.0, 0.0);
        let f = dot(normalize(to_light), normal_world);
        color_out = vec4<f32>(f * vec3<f32>(1.0, 0.0, 0.0), 1.0);
        // color_out *= light_strength / dot(to_light, to_light);
    }
  
    textureStore(output_texture, id.xy, color_out);
}

fn trace_ray_tlas(ray: ptr<function, Ray>) {
    traverse_tlas(ray);
}

fn trace_ray(ray: ptr<function, Ray>)  {
    for (var i = 0u; i < scene_uniform.instance_count; i += 1u) {
        traverse_blas(ray, i);
    }

}

// fn trace_ray(ray: ptr<function, Ray>)  {
//     for (var i = 0u; i < scene_uniform.instance_count; i += 1u) {
//         let instance = instances[i];
//         // Transform to object space
//         var ray_object = *ray;
//         let origin_object = instance.world_object * vec4f(ray_object.origin, 1.0);
//         ray_object.origin = origin_object.xyz / origin_object.w;
//         ray_object.dir = normalize((instance.world_object * vec4f(ray_object.dir, 0.0)).xyz);
//         traverse_blas(&ray_object, i);

//         if ray_object.record.t < (*ray).record.t {
//             // Back to world space
//             let world_hit_position_h = instance.object_world * vec4f(ray_object.origin + ray_object.record.t * ray_object.dir, 1.0);
//             let world_hit_position = world_hit_position_h.xyz / world_hit_position_h.w;
//             var world_normal = normalize((instance.object_world * vec4f(ray_object.record.normal, 0.0)).xyz);
//             (*ray).record.t = length(world_hit_position - (*ray).origin);
//             (*ray).record.position = world_hit_position;
//             (*ray).record.normal = normalize(world_normal);
//         }
//     }
// }

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
    var ray_object = *ray;
    let origin_object = instance.world_object * vec4f(ray_object.origin, 1.0);
    ray_object.origin = origin_object.xyz / origin_object.w;
    ray_object.dir = normalize((instance.world_object * vec4f(ray_object.dir, 0.0)).xyz);
    // ray_object.dir = (instance.world_object * vec4f(ray_object.dir, 0.0)).xyz;

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

    (*ray).record = ray_object.record;

    // if ray_object.record.t < (*ray).record.t {
    //     // Transform ray back to world space.
    //     let world_hit_position_h = instance.object_world * vec4f(ray_object.origin + ray_object.record.t * ray_object.dir, 1.0);
    //     let world_hit_position = world_hit_position_h.xyz / world_hit_position_h.w;
    //     var world_normal = normalize((instance.object_world * vec4f(ray_object.record.normal, 0.0)).xyz);
    //     (*ray).record.t = length(world_hit_position - (*ray).origin);
    //     (*ray).record.position = world_hit_position;
    //     (*ray).record.normal = normalize(world_normal);
    // }
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
    let edge_1 = prim.p1 - prim.p0;
    let edge_2 = prim.p2 - prim.p0;
    let h = cross((*ray).dir, edge_2);
    let a = dot(edge_1, h);
    // if a > -0.0001 && a < 0.0001 { // Ray parallel to triangle
    if abs(a) < 0.0001 { // Ray parallel to triangle
        return;
    }
    let f = 1.0 / a;
    let s = (*ray).origin - prim.p0;
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
