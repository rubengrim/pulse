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
    let t_far = 100000.0;
    ray.record = HitRecord(t_far, vec3<f32>(0.0, 0.0, 0.0));

    var color_out: vec4<f32>;
    let light_position = vec3<f32>(2.0, 5.0, 0.0);
    let light_strength = 10.0;
    trace_ray(&ray);
    if ray.record.t < 0.1 || ray.record.t >= t_far  {
        // Miss
        color_out = vec4<f32>(0.0, 0.3, 0.3, 1.0);
    } else {
        // Hit
        // let a = (*(&vertex_buffers[0]).buffer)[0];
        // color_out = vec4<f32>(a, a, a, 0.0);

        let hit_position = ray.origin + ray.record.t * ray.dir;
        let to_light = light_position - hit_position;
        let surface_normal = normalize(ray.record.normal);
        let f = dot(normalize(to_light), surface_normal);
        color_out = vec4<f32>(f * vec3<f32>(1.0, 0.0, 0.0), 1.0);
        color_out *= light_strength / (length(to_light) * length(to_light));
    }

    // var color_out: vec4<f32>;
    // let light_position = vec3<f32>(2.0, 2.0, 0.0);
    // // trace_ray(&ray);
    // ray_sphere_intersect(&ray, vec3<f32>(0.0, 0.0, 0.0), 1.0);
    // if ray.record.t < 0.1 || ray.record.t >= t_far {
    //     // Miss
    //     color_out = vec4<f32>(0.0, 0.3, 0.3, 1.0);
    // } else {
    //     // Hit
    //     // let a = (*(&vertex_buffers[0]).buffer)[0];
    //     // color_out = vec4<f32>(a, a, a, 0.0);

    //     let hit_position = ray.origin + ray.record.t * ray.dir;
    //     let to_light = normalize(light_position - hit_position);
    //     let surface_normal = normalize(ray.record.normal);
    //     let f = dot(to_light, surface_normal);
    //     color_out = vec4<f32>(f * vec3<f32>(1.0, 0.0, 0.0), 1.0);
    // }

    // var c: vec4<f32>;
    // if length(ray_target.xy) < 0.3 {
    
    textureStore(output_texture, id.xy, color_out);
}

fn trace_ray(ray: ptr<function, Ray>)  {
    for (var i = 0; i < 1536; i += 1) {
        ray_triangle_intersect(ray, triangles[i]);
    }
}

fn ray_bvh_intersect(ray_origin: vec3<f32>, ray_dir: vec3<f32>, ray_t: ptr<function, f32>) {
    
}

// fn ray_aabb_intersect(ray_origin: vec3<f32>, ray_dir: vec3<f32>, ray_t: f32, aabb_min: vec3<f32>, aabb_max: vec3<f32>) -> bool {
//     let t_x_1 = (aabb_min.x - ray_origin.x) / ray_direction.x;
//     let t_x_2 = (aabb_max.x - ray_origin.x) / ray_direction.x;
//     var t_min = min(t_x_1, t_x_2); 
//     var t_max = max(t_x_1, t_x_2); 

//     let t_y_1 = (aabb_min.y - ray_origin.y) / ray_direction.y;
//     let t_y_2 = (aabb_max.y - ray_origin.y) / ray_direction.y;
//     t_min = min(t_min, min(t_y_1, t_y_2)); 
//     t_max = max(t_max, max(t_y_1, t_y_2)); 

//     let t_z_1 = (aabb_min.z - ray_origin.z) / ray_direction.z;
//     let t_z_2 = (aabb_max.z - ray_origin.z) / ray_direction.z;
//     t_min = min(t_min, min(t_z_1, t_z_2)); 
//     t_max = max(t_max, max(t_z_1, t_z_2)); 

//     return t_max >= t_min && t_min < ray_t && t_max > 0.0;
// }


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
