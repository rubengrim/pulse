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
    aabb_max: vec3<f32>,
    // child_b_idx is always child_a_idx + 1 so don't store it.
    child_a_idx: u32,
    // Index for tri_indices, not directly for the tri data.
    first_primitive: u32,
    primitive_count: u32,
}


@group(0) @binding(0) var<storage> triangles: array<PulseTriangle>;
@group(0) @binding(1) var<storage> triangle_indices: array<u32>;
@group(0) @binding(2) var<storage> nodes: array<BVHNode>;

@group(1) @binding(0) var<uniform> view: View;
@group(1) @binding(1) var output_texture: texture_storage_2d<rgba16float, read_write>;

struct Ray {
    origin: vec3<f32>,
    dir: vec3<f32>,
    record: HitRecord,
}

struct HitRecord {
    t: f32,
    normal: vec3<f32>,
}

@compute @workgroup_size(8, 8, 1)
fn path_trace(@builtin(global_invocation_id) id: vec3<u32>) {
    var pixel_uv = (vec2<f32>(id.xy) + 0.5) / view.viewport.zw;
    // Clip position goes from -1 to 1.
    let pixel_clip_pos = (pixel_uv * 2.0) - 1.0;
    let ray_target = view.inverse_view_proj * vec4<f32>(pixel_clip_pos.x, -pixel_clip_pos.y, 1.0, 1.0);
    var ray = Ray();
    ray.origin = view.world_position;
    ray.dir = normalize((ray_target.xyz / ray_target.w) - ray.origin);
    let t_far = 1e30;
    ray.record = HitRecord(t_far, vec3<f32>(0.0, 0.0, 0.0));

    var color_out: vec4<f32>;
    let light_position = vec3<f32>(2.0, 5.0, 1.0);
    let light_strength = 15.0;
    trace_ray(&ray);
    if ray.record.t < 0.1 || ray.record.t >= t_far  {
        // Miss
        color_out = vec4<f32>(0.0, 0.3, 0.3, 1.0);
    } else {
        // Hit
        let hit_position = ray.origin + ray.record.t * ray.dir;
        let to_light = light_position - hit_position;
        let surface_normal = normalize(ray.record.normal);
        let f = dot(normalize(to_light), surface_normal);
        color_out = vec4<f32>(f * vec3<f32>(1.0, 0.0, 0.0), 1.0);
        color_out *= light_strength / (length(to_light) * length(to_light));
    }
  
    textureStore(output_texture, id.xy, color_out);
}

fn trace_ray(ray: ptr<function, Ray>)  {
    ray_bvh_intersect(ray);
    // (*ray).record.t = ray_aabb_intersect(ray, vec3<f32>(-1.0, -1.0, -4.0), vec3<f32>(1.0, 1.0, -2.0));
    // (*ray).record.t = 10.0;
}

fn ray_bvh_intersect(ray: ptr<function, Ray>) {
    var node = 0u;
    var stack: array<u32, 64>;
    var stack_ptr = 0;
    var iteration = 0;
    let max_iterations = 1000000;
    while iteration < max_iterations {
        iteration += 1;
        if nodes[node].primitive_count > 0u {
            for (var i: u32 = 0u; i < nodes[node].primitive_count; i += 1u) {
                let tri_idx = triangle_indices[nodes[node].first_primitive + i];
                ray_triangle_intersect(ray, triangles[tri_idx]);
            }
            if stack_ptr == 0 {
                break;
            } else {
                stack_ptr -= 1;
                node = stack[stack_ptr];
            }
            continue;
        }

        var child_a_idx = nodes[node].child_a_idx;
        var child_b_idx = child_a_idx + 1u;
        var dist_a = ray_aabb_intersect(ray, nodes[child_a_idx].aabb_min, nodes[child_a_idx].aabb_max);
        var dist_b = ray_aabb_intersect(ray, nodes[child_b_idx].aabb_min, nodes[child_b_idx].aabb_max);
        if dist_a > dist_b {
            let d = dist_a;
            dist_a = dist_b;
            dist_b = d;
            let c = child_a_idx;
            child_a_idx = child_b_idx;
            child_b_idx = c;
        }
        if dist_a == 1e30f {
            if stack_ptr == 0 {
                break;
            } else  {
                stack_ptr -= 1;
                node = stack[stack_ptr];
            }
        } else {
            node = child_a_idx;
            if dist_b != 1e30f {
                stack[stack_ptr] = child_b_idx;
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
