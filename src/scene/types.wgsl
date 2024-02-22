#define_import_path pulse::scene::types

struct SceneUniform {
    instance_count: u32,
    light_count: u32,
}

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

struct Primitive {
    p_first: vec3<f32>,
    p_second: vec3<f32>,
    p_third: vec3<f32>,
}

struct TriangleData {
    n_first: vec3<f32>,
    n_second: vec3<f32>,
    n_third: vec3<f32>,
    uv_first: vec2<f32>,
    uv_second: vec2<f32>,
    uv_third: vec2<f32>,
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
    triangle_count: u32,
    index_offset: u32,
    node_offset: u32,
    material_index: u32,
}

struct Material {
    base_color: vec4f,
    emissive: vec4f,
    perceptual_roughness: f32,
    reflectance: f32,
    metallic: f32,
}

struct LightDataIndex {
    // Offset into `light_emission_strength_cdf`. Count is obtained from `MeshInstance::triangle_count`.
    cdf_offset: u32,
    mesh_instance_index: u32,
}
