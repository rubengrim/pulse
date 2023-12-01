#define_import_path pulse::scene_bindings

struct PulseTriangle {
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    n0: Vec3,
    n1: Vec3,
    n2: Vec3,
    uv0: Vec2,
    uv1: Vec2,
    uv2: Vec2,
}

struct BVHNode {
    aabb_min: Vec3,
    aabb_max: Vec3,
    // child_b_idx is always child_a_idx + 1 so don't store it.
    child_a_idx: u32,
    // Index for tri_indices, not directly for the tri data.
    first_primitive: u32,
    primitive_count: u32,
}

@binding(0) @location(0) var<storage> triangles: array<PulseTriangle>;
@binding(0) @location(1) var<storage> triangle_indices: array<u32>;
@binding(0) @location(2) var<storage> nodes: array<BVHNode>;
