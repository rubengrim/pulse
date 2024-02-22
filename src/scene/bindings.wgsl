#define_import_path pulse::scene::bindings

#import pulse::scene::types::{
    Primitive,
    TriangleData,
    BLASNode,
    TLASNode,
    MeshInstance,
    SceneUniform,
    Material,
    LightDataIndex,
}

@group(0) @binding(0) var<uniform> scene_uniform: SceneUniform;
@group(0) @binding(1) var<storage> primitives: array<Primitive>;
@group(0) @binding(2) var<storage> triangle_data: array<TriangleData>;
@group(0) @binding(3) var<storage> triangle_indices: array<u32>;
@group(0) @binding(4) var<storage> blas_nodes: array<BLASNode>;
@group(0) @binding(5) var<storage> tlas_nodes: array<TLASNode>;
@group(0) @binding(6) var<storage> instance_indices: array<u32>;
@group(0) @binding(7) var<storage> instances: array<MeshInstance>;
@group(0) @binding(8) var<storage> materials: array<Material>;
// CDF used for sampling light source based on emission strength
@group(0) @binding(9) var<storage> light_emission_strength_cdf: array<f32>;
// Contains all CDFs used for uniformly sampling a light source triangles. Indexed by `light_indices`
@group(0) @binding(10) var<storage> light_triangle_area_cdfs: array<f32>;
@group(0) @binding(11) var<storage> light_mesh_areas: array<f32>;
@group(0) @binding(12) var<storage> light_indices: array<LightDataIndex>;

