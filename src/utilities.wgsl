#define_import_path pulse::utils

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
        blue_noise_texture,
    }
}

const PI: f32 = 3.14159265358;
const HALF_PI: f32 = 1.57079632679;
const TWO_PI: f32 = 6.28318530718;
const INV_PI: f32 = 0.31830988618;

//------------
// BEGIN: MISC

fn length_sq(v: vec3f) -> f32 {
    return v.x * v.x + v.y * v.y + v.z * v.z;
}

fn distance_sq(v1: vec3f, v2: vec3f) -> f32 {
    return length_sq(v1 - v2);
}

fn clamp_v(v: vec3f, min: f32, max: f32) -> vec3f {
    return vec3f(clamp(v.x, min, max), clamp(v.y, min, max), clamp(v.z, min, max));
}

// Assumes a and b are unit length.
fn are_aligned(a: vec3f, b: vec3f, threshold: f32) -> bool {
    return length(a - b) < threshold;
}

fn are_close(a: f32, b: f32, threshold: f32) -> bool {
    return abs(a - b) < threshold;
}

// END: MISC
//----------

//--------------------------------
// BEGIN: RANDOM NUMBER GENERATION

// https://developer.nvidia.com/blog/rendering-in-real-time-with-spatiotemporal-blue-noise-textures-part-2/
fn next_bn_sample(rng_state: ptr<function, u32>, bn_texture_offset: vec2u, pixel_id: vec2u) -> f32 {
    let texture_coord = bn_texture_offset + ((pixel_id + vec2u(rand_f_pair(rng_state) * 8.0) * 8u) % 64u);
    return textureLoad(blue_noise_texture, texture_coord, 0).x;
}

fn rand_u(state: ptr<function, u32>) -> u32 {
    // PCG hash
    *state = *state * 747796405u + 2891336453u;
    let word = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn rand_range_u(n: u32, state: ptr<function, u32>) -> u32 {
    return rand_u(state) % n;
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

// END: RANDOM NUMBER GENERATION
//------------------------------

//-------------------------------------
// BEGIN: RAY ACCELERATION/INTERSECTION

fn get_blas_node(index: u32, instance_index: u32) -> BLASNode {
    let instance = instances[instance_index];
    return blas_nodes[index + instance.node_offset];
}

fn get_primitive(index: u32, instance_index: u32) -> Primitive {
    let instance = instances[instance_index];
    let triangle_index = triangle_indices[index + instance.index_offset];
    return primitives[triangle_index + instance.triangle_offset];
}

fn get_triangle_data(index: u32, instance_index: u32) -> TriangleData {
    let instance = instances[instance_index];
    let triangle_index = triangle_indices[index + instance.index_offset];
    return triangle_data[triangle_index + instance.triangle_offset];
}

fn trace_ray(ray: ptr<function, Ray>) {
    traverse_tlas(ray);
}

// Returns whether or not the ray hit anything within `distance_threshold`
fn trace_shadow_ray(ray: ptr<function, Ray>, distance_threshold: f32) -> bool {
    return traverse_tlas_for_shadow_ray(ray, distance_threshold);
}

fn trace_ray_blas_only(ray: ptr<function, Ray>)  {
    for (var i = 0u; i < scene_uniform.instance_count; i += 1u) {
        traverse_blas(ray, i);
    }
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

// Returns whether or not the ray hit anything within `distance_threshold`.
fn traverse_tlas_for_shadow_ray(ray: ptr<function, Ray>, distance_threshold: f32) -> bool {
    // Abort on empty/invalid root node.
    if tlas_nodes[0].a_or_first_instance == 0u && tlas_nodes[0].instance_count == 0u {
        return false;
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
                if traverse_blas_for_shadow_ray(ray, instance_index, distance_threshold) {
                    return true;
                }
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

    return false;
}

// Takes world space ray.
fn traverse_blas(ray: ptr<function, Ray>, instance_index: u32) {
    // Transform ray to object/blas space.
    let instance = instances[instance_index];
    var ray_object = Ray();
    ray_object.origin = transform_position(instance.world_object, (*ray).origin);
    ray_object.dir = normalize(transform_direction(instance.world_object, (*ray).dir));
    ray_object.record = RayHitRecord(1e30, 0u, 0u, 0.0, 0.0);

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

// Returns whether or not the ray hit anything within `distance_threshold`
// Takes world space ray
fn traverse_blas_for_shadow_ray(ray: ptr<function, Ray>, instance_index: u32, distance_threshold: f32) -> bool {
    // Transform ray to object/blas space
    let instance = instances[instance_index];
    var ray_object = Ray();
    ray_object.origin = transform_position(instance.world_object, (*ray).origin);
    ray_object.dir = normalize(transform_direction(instance.world_object, (*ray).dir));
    ray_object.record = RayHitRecord(1e30, 0u, 0u, 0.0, 0.0);

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

    if new_t_world <= distance_threshold {
        return true;
    }

    if new_t_world < (*ray).record.t {
        (*ray).record = ray_object.record;
        (*ray).record.t = new_t_world;
    }

    return false;
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

// END: RAY ACCELERATION/INTERSECTION
//-----------------------------------

//----------------
// BEGIN: SAMPLING

fn sample_cosine_hemisphere(normal: vec3<f32>, e0: f32, e1: f32) -> vec3f {
    let cos_theta = 2.0 * e0 - 1.0;
    let phi = 2.0 * PI * e1;
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let sin_phi = sin(phi);
    let cos_phi = cos(phi);
    let unit_sphere_direction = normalize(vec3(sin_theta * cos_phi, cos_theta, sin_theta * sin_phi));
    return normalize(normal + unit_sphere_direction);
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

// END: SAMPLING
//--------------

//-----------------------
// BEGIN: TRANSFORMATIONS

// Orthonormal basis vectors
struct ON {
    e_one: vec3f,
    e_two: vec3f,
    e_three: vec3f,
}

// Produces right-handed with `e2` aligned to `normal`
fn orthonormal_from_normal(normal: vec3f) -> ON {
    let e_two = normal;
    var e_three: vec3f;
    if !are_aligned(e_two, vec3f(1.0, 0.0, 0.0), 0.001) {
        e_three = normalize(cross(e_two, vec3f(1.0, 0.0, 0.0)));
    } else {
        e_three = normalize(cross(e_two, vec3f(0.0, 1.0, 0.0)));
    }
    let e_one = normalize(cross(e_two, e_three));
    return ON(e_one, e_two, e_three);
}

// Assumes right-handed with `e2` up
fn spherical_to_cartesian_in_on(theta: f32, phi: f32, on: ON) -> vec3f {
    let sin_theta = sin(theta);
    return 
        (on.e_one * sin_theta * sin(phi))
        + (on.e_two * cos(theta)
        + (on.e_three * sin_theta * cos(phi))
    );
}

// right-handed with y-axis up
fn spherical_to_cartesian(theta: f32, phi: f32) -> vec3f {
    let sin_theta = sin(theta);
    return vec3f(
        sin_theta * sin(phi),
        cos(theta),
        sin_theta * cos(phi),
    );
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

// Reflects `a` about `b`. Assumes `b` is normalized.
fn reflect(a: vec3f, b: vec3f) -> vec3f {
    return 2.0 * dot(a, b) * b - a;
}

// END: TRANSFORMATIONS
//---------------------

//-----------
// BEGIN: GGX

// from https://agraphicsguynotes.com/posts/sample_microfacet_brdf/
fn pdf_D_GGX(view: vec3f, half: vec3f, theta: f32, phi: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let cos_theta = cos(theta);
    let denominator = 2.0 * a * cos_theta * sin(theta);
    let d = (a - 1.0) * cos_theta + 1.0;
    let numerator = 4.0 * dot(view, half) * d * d;
    return denominator / numerator;
}

fn fresnel_schlick(cos_theta: f32, f0: vec3f) -> vec3f {
    return f0 + (1.0 - f0) * pow(1.0 - cos_theta, 5.0);
}

fn D_GGX(NdotM: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let NdotM2= NdotM * NdotM;
    let b = (NdotM2* (a2 - 1.0) + 1.0);
    return a2 * INV_PI / (b * b);
}

fn G1_GGX_schlick(NdotO: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let k = a / 2.0;
    return NdotO / (NdotO * (1.0 - k) + k);
}

fn G_smith(NdotO: f32, NdotI: f32, roughness: f32) -> f32 {
    return G1_GGX_schlick(NdotI, roughness) * G1_GGX_schlick(NdotO, roughness);
}

struct ImportanceSamplingResult {
    // Sampled direction in world space.
    wi: vec3f,
    // This is what the light coming from `wi` should be multiplied by.
    reflectance: vec3f,
}

fn importance_sample_ggx_d(n: vec3f, wo: vec3f, material: Material, rng_state: ptr<function, u32>, bn_texture_offset: vec2u, pixel_id: vec2u) -> ImportanceSamplingResult {
    var f0 = vec3f(0.16 * material.reflectance * material.reflectance);
    f0 = f0 * (1.0 - material.metallic) + material.base_color.rgb * material.metallic;

    let NdotO = dot(n, wo);
    let F0 = fresnel_schlick(NdotO, f0);
    let F0_max = max(max(F0.r, F0.g), F0.b);

    let e0 = rand_f(rng_state);
    let e1 = rand_f(rng_state);
    let e2 = rand_f(rng_state);

    // let e0 = next_bn_sample(rng_state, bn_texture_offset, pixel_id);
    // let e1 = next_bn_sample(rng_state, bn_texture_offset, pixel_id);
    // let e2 = next_bn_sample(rng_state, bn_texture_offset, pixel_id);

    var brdf: vec3f;
    var pdf_s: f32;
    var pdf_d: f32;
    var wi: vec3f;
    if e0 <= F0_max {
        // Sample specular
        let a = material.perceptual_roughness * material.perceptual_roughness;
        let a2 = a * a;
        let theta = acos(sqrt((1.0 - e1) / (e1 * (a2 - 1.0) + 1.0)));
        let phi = e2 * TWO_PI;

        // Create tangent space basis vectors.
        let on = orthonormal_from_normal(n);
        // Microsurface normal or half-vector in world space.
        let wm = spherical_to_cartesian_in_on(theta, phi, on);
        // Incident direction in world space.
        wi = reflect(wo, wm);

        let NdotM = dot(n, wm);
        let NdotI = dot(n, wi);
        let OdotM = dot(wo, wm);

        let F = fresnel_schlick(OdotM, f0);
        let D = D_GGX(NdotM, material.perceptual_roughness);
        let G = G_smith(NdotO, NdotI, material.perceptual_roughness);

        brdf = (F * D * G) / (4.0 * NdotO * NdotI);
        pdf_s = D * NdotM / (4.0 * OdotM);
    } else {
        // Sample diffuse
        wi = sample_cosine_hemisphere(n, e1, e2);
        let NdotI = dot(n, wi);
        brdf = material.base_color.rgb * PI * NdotI;
        pdf_d = PI * NdotI;
    }

    let pdf = ((1.0 - F0_max) * pdf_d) + (F0_max * pdf_s);
    let reflectance = brdf / pdf;

    return ImportanceSamplingResult(wi, reflectance);
}

// END: GGX
//---------
