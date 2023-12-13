// #import pulse::scene_bindings:
#import bevy_render::view::View

struct PulseTriangle {
    p0: vec3<f32>,
    p1: vec3<f32>,
    p2: vec3<f32>,
    n0: vec3<f32>,
    n1: vec3<f32>,
    n2: vec3<f32>,
    uv0: vec2<f32>,
    uv1: vec2<f32>,
    uv2: vec2<f32>,
}

struct BVHNode {
    aabb_min: vec3<f32>,
    a_or_tri: u32,
    aabb_max: vec3<f32>,
    tri_count: u32,
}

struct PulseMeshInstance {
    object_world: mat4x4f,
    world_object: mat4x4f,
    triangle_offset: u32,
    // triangle_count: u32,
    index_offset: u32,
    // index_count: u32,
    node_offset: u32,
    // node_count: u32,
}

struct PulseSceneUniform {
    instance_count: u32,
}

@group(0) @binding(0) var<uniform> scene_uniform: PulseSceneUniform;
@group(0) @binding(1) var<storage> triangles: array<PulseTriangle>;
@group(0) @binding(2) var<storage> triangle_indices: array<u32>;
@group(0) @binding(3) var<storage> nodes: array<BVHNode>;
@group(0) @binding(4) var<storage> instances: array<PulseMeshInstance>;

@group(1) @binding(0) var<uniform> view: View;
@group(1) @binding(1) var output_texture: texture_storage_2d<rgba16float, read_write>;

struct Ray {
    origin: vec3f,
    dir: vec3f,
    record: ObjectHitRecord,
}

struct ObjectHitRecord {
    t: f32,
    position: vec3f,
    normal: vec3f,
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
    ray.record = ObjectHitRecord(t_far, vec3f(0.0, 0.0, 0.0), vec3f(0.0, 0.0, 0.0));

    var color_out: vec4<f32>;
    let light_position = vec3<f32>(2.0, 5.0, 1.0);
    let light_strength = 15.0;
    trace_ray(&ray);
    if ray.record.t < 0.1 || ray.record.t >= t_far  {
        // Miss
        color_out = vec4<f32>(0.0, 0.3, 0.3, 1.0);
    } else {
        // Hit
        let to_light = light_position - ray.record.position;
        let f = dot(normalize(to_light), ray.record.normal);
        color_out = vec4<f32>(f * vec3<f32>(1.0, 0.0, 0.0), 1.0);
        color_out *= light_strength / dot(to_light, to_light);
    }
  
    textureStore(output_texture, id.xy, color_out);
}

fn trace_ray(ray: ptr<function, Ray>)  {
    for (var i = 0u; i < scene_uniform.instance_count; i += 1u) {
        let instance = instances[i];
        // Transform to object space
        var ray_object = *ray;
        let origin_object = instance.world_object * vec4f(ray_object.origin, 1.0);
        ray_object.origin = origin_object.xyz / origin_object.w;
        ray_object.dir = normalize((instance.world_object * vec4f(ray_object.dir, 0.0)).xyz);
        traverse_bvh(&ray_object, i);

        if ray_object.record.t < (*ray).record.t {
            // Back to world space
            let world_hit_position_h = instance.object_world * vec4f(ray_object.origin + ray_object.record.t * ray_object.dir, 1.0);
            let world_hit_position = world_hit_position_h.xyz / world_hit_position_h.w;
            var world_normal = normalize((instance.object_world * vec4f(ray_object.record.normal, 0.0)).xyz);
            (*ray).record.t = length(world_hit_position - (*ray).origin);
            (*ray).record.position = world_hit_position;
            (*ray).record.normal = normalize(world_normal);
        }
    }

}

fn get_node(index: u32, instance_index: u32) -> BVHNode {
    let instance = instances[instance_index];
    return nodes[index + instance.node_offset];
}

fn get_triangle(index: u32, instance_index: u32) -> PulseTriangle {
    let instance = instances[instance_index];
    let triangle_index = triangle_indices[index + instance.index_offset];
    return triangles[triangle_index + instance.triangle_offset];
}

fn traverse_bvh(ray: ptr<function, Ray>, instance_index: u32) {
    var node_index = 0u;
    var stack: array<u32, 64>;
    var stack_ptr = 0;
    var iteration = 0;
    let max_iterations = 100000;
    while iteration < max_iterations {
        iteration += 1;
        let node = get_node(node_index, instance_index);
        if node.tri_count > 0u {
            for (var i: u32 = 0u; i < node.tri_count; i += 1u) {
                ray_triangle_intersect(ray, get_triangle(node.a_or_tri + i, instance_index));
            }
            if stack_ptr == 0 {
                break;
            } else {
                stack_ptr -= 1;
                node_index = stack[stack_ptr];
            }
            continue;
        }

        var child_a_index = node.a_or_tri;
        var child_b_index = child_a_index + 1u;
        let child_a = get_node(child_a_index, instance_index);
        let child_b = get_node(child_b_index, instance_index);
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

// fn raySphereIntersect(vec3 r0, vec3 rd, vec3 s0, float sr) {
//     // - r0: ray origin
//     // - rd: normalized ray direction
//     // - s0: sphere center
//     // - sr: sphere radius
//     // - Returns distance from r0 to first intersecion with sphere,
//     //   or -1.0 if no intersection.
//     float a = dot(rd, rd);
//     vec3 s0_r0 = r0 - s0;
//     float b = 2.0 * dot(rd, s0_r0);
//     float c = dot(s0_r0, s0_r0) - (sr * sr);
//     if (b*b - 4.0*a*c < 0.0) {
//         return -1.0;
//     }
//     return (-b - sqrt((b*b) - 4.0*a*c))/(2.0*a);
// }

fn ray_sphere_intersect(ray: ptr<function, Ray>, s0: vec3<f32>, sr: f32){
    // let t = 1.0;
    // (*ray).record.t = t;
    // let hit_pos = (*ray).origin + t * (*ray).dir;
    // (*ray).record.normal = normalize(hit_pos - s0);

    let a = dot((*ray).dir, (*ray).dir);
    let s0_r0 = (*ray).origin - s0;
    let b = 2.0 * dot((*ray).dir, s0_r0);
    let c = dot(s0_r0, s0_r0) - (sr * sr);
    if (b*b - 4.0*a*c >= 0.0) {
        let t = (-b - sqrt((b*b) - 4.0*a*c))/(2.0*a);
        if (t > 0.001 && (*ray).record.t > t) {
            (*ray).record.t = t;
            let hit_pos = (*ray).origin + t * (*ray).dir;
            (*ray).record.normal = normalize(hit_pos - s0);
        }
    }
}

// Updates ray_t if new t is smaller
fn ray_triangle_intersect(ray: ptr<function, Ray>, tri: PulseTriangle) {
    let edge_1 = tri.p1 - tri.p0;
    let edge_2 = tri.p2 - tri.p0;
    let h = cross((*ray).dir, edge_2);
    let a = dot(edge_1, h);
    if a > -0.0001 && a < 0.0001 { // Ray parallel to triangle
        return;
    }
    let f = 1.0 / a;
    let s = (*ray).origin - tri.p0;
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
    if t > 0.001 && (*ray).record.t > t {
        (*ray).record.t = t;
        (*ray).record.normal = tri.n0;
    }
}
