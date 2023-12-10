use super::PulseTriangle;
use bevy::{prelude::*, render::render_resource::ShaderType};

#[derive(Default, ShaderType, Clone, Debug)]
pub struct PulseBLASNode {
    pub aabb_min: Vec3,
    pub aabb_max: Vec3,
    // child_b_idx is always child_a_idx + 1 so don't store it.
    pub child_a_idx: u32,
    // Index for tri_indices, not directly for the tri data.
    pub first_tri: u32,
    pub tri_count: u32,
}

pub struct Blas {
    pub nodes: Vec<PulseBLASNode>,
    pub tri_indices: Vec<u32>,
}

#[derive(Default, Copy, Clone)]
pub struct AABB {
    min: Vec3,
    max: Vec3,
}

impl AABB {
    pub fn grow_position(&mut self, p: Vec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    pub fn grow_aabb(&mut self, aabb: AABB) {
        self.grow_position(aabb.min);
        self.grow_position(aabb.max);
    }

    pub fn area(&self) -> f32 {
        let e = self.max - self.min;
        e.x * e.y + e.y * e.z + e.z * e.x
    }
}

#[derive(Default, Copy, Clone)]
struct Bin {
    bounds: AABB,
    tri_count: u32,
}

pub fn build_blas(tris: &Vec<PulseTriangle>) -> Blas {
    let mut tri_indices: Vec<usize> = vec![];
    // TODO: Use AABB centers instead of triangle centroids.
    let mut centroids: Vec<Vec3> = vec![];
    for i in 0..tris.len() {
        centroids
            .push((tris[i].positions[0] + tris[i].positions[1] + tris[i].positions[2]) * 0.3333);
        tri_indices.push(i);
    }

    let mut nodes: Vec<PulseBLASNode> = vec![];
    let mut root = PulseBLASNode::default();
    root.child_a_idx = 0;
    root.first_tri = 0;
    root.tri_count = tri_indices.len() as u32;
    calculate_node_aabb(&mut root, tris, &tri_indices);
    nodes.push(root);

    subdivide(0, &mut nodes, tris, &centroids, &mut tri_indices);

    // Ugly fix. Should already use u32
    let tri_indices = tri_indices.iter().map(|i| *i as u32).collect::<Vec<u32>>();

    Blas { nodes, tri_indices }
}

pub fn subdivide(
    node_idx: usize,
    nodes: &mut Vec<PulseBLASNode>,
    tris: &Vec<PulseTriangle>,
    centroids: &Vec<Vec3>,
    tri_indices: &mut Vec<usize>,
) {
    if nodes[node_idx].tri_count <= 4 {
        return;
    }

    let (axis, split_position, split_cost) =
        find_best_split_plane(&nodes[node_idx], tris, centroids, tri_indices);

    let no_split_cost = calculate_node_cost(&nodes[node_idx]);
    if split_cost >= no_split_cost {
        return;
    }

    let mut i = nodes[node_idx].first_tri;
    let mut j = i + nodes[node_idx].tri_count - 1;
    while i <= j {
        if centroids[tri_indices[i as usize]][axis] < split_position {
            i += 1;
        } else {
            swap(tri_indices, i as usize, j as usize);
            j -= 1;
        }
    }

    let a_count = i - nodes[node_idx].first_tri;
    // Don't split the nodes[node_idx] if either one of it's children contain no primitives.
    if a_count == 0 || a_count == nodes[node_idx].tri_count {
        return;
    }

    let node_count = nodes.len() as u32;

    let mut child_a = PulseBLASNode::default();
    child_a.first_tri = nodes[node_idx].first_tri;
    child_a.tri_count = a_count;
    calculate_node_aabb(&mut child_a, tris, tri_indices);
    nodes.push(child_a);

    let mut child_b = PulseBLASNode::default();
    child_b.first_tri = i;
    child_b.tri_count = nodes[node_idx].tri_count - a_count;
    calculate_node_aabb(&mut child_b, tris, tri_indices);
    nodes.push(child_b);

    nodes[node_idx].child_a_idx = node_count;
    // Parent nodes[node_idx] is not a leaf, so set prim count to 0.
    nodes[node_idx].tri_count = 0;

    subdivide(
        nodes[node_idx].child_a_idx as usize,
        nodes,
        tris,
        centroids,
        tri_indices,
    );
    subdivide(
        nodes[node_idx].child_a_idx as usize + 1,
        nodes,
        tris,
        centroids,
        tri_indices,
    );
}

// Returns (axis, position, cost)
fn find_best_split_plane(
    node: &PulseBLASNode,
    tris: &Vec<PulseTriangle>,
    centroids: &Vec<Vec3>,
    tri_indices: &Vec<usize>,
) -> (usize, f32, f32) {
    let mut best_axis = 0;
    let mut best_position = 0.0;
    let mut best_cost = 1e30;
    for axis in 0..3 {
        let mut bounds_min: f32 = 1e30;
        let mut bounds_max: f32 = -1e30;
        for i in 0..node.tri_count {
            bounds_min = bounds_min.min(centroids[tri_indices[i as usize]][axis]);
            bounds_max = bounds_max.max(centroids[tri_indices[i as usize]][axis]);
        }
        if bounds_min == bounds_max {
            continue;
        }

        // Create bins
        const BIN_COUNT: usize = 20;
        let mut bins: [Bin; BIN_COUNT] = [Bin::default(); BIN_COUNT];
        let bin_size_inv = BIN_COUNT as f32 / (bounds_max - bounds_min);
        for i in 0..node.tri_count {
            let triangle = &tris[tri_indices[(node.first_tri + i) as usize]];
            let bin_idx = (BIN_COUNT - 1).min(
                ((centroids[tri_indices[(node.first_tri + i) as usize]][axis] - bounds_min)
                    * bin_size_inv) as usize,
            );
            bins[bin_idx].tri_count = bins[bin_idx].tri_count + 1;
            bins[bin_idx].bounds.grow_position(triangle.positions[0]);
            bins[bin_idx].bounds.grow_position(triangle.positions[1]);
            bins[bin_idx].bounds.grow_position(triangle.positions[2]);
        }

        // Calculate bin data
        let mut area_a = [0.0; BIN_COUNT - 1];
        let mut area_b = [0.0; BIN_COUNT - 1];
        let mut count_a = [0u32; BIN_COUNT - 1];
        let mut count_b = [0u32; BIN_COUNT - 1];
        let mut box_a = AABB::default();
        let mut box_b = AABB::default();
        let mut sum_a = 0;
        let mut sum_b = 0;
        for i in 0..(BIN_COUNT - 1) {
            sum_a += bins[i].tri_count;
            count_a[i] = sum_a;
            box_a.grow_aabb(bins[i].bounds);
            area_a[i] = box_a.area();

            sum_b += bins[BIN_COUNT - 1 - i].tri_count;
            count_b[BIN_COUNT - 2 - i] = sum_b;
            box_b.grow_aabb(bins[BIN_COUNT - 1 - i].bounds);
            area_b[BIN_COUNT - 2 - i] = box_b.area();
        }

        // Evaluate SAH for the planes between the bins
        let bin_size = (bounds_max - bounds_min) / BIN_COUNT as f32;
        for i in 0..(BIN_COUNT - 1) {
            let plane_cost = count_a[i] as f32 * area_a[i] + count_b[i] as f32 * area_b[i];
            if plane_cost < best_cost {
                best_axis = axis;
                best_position = bounds_min + bin_size * (i + 1) as f32;
                best_cost = plane_cost;
            }
        }

        let num_steps = 10;
        let step_size = (bounds_max - bounds_min) / num_steps as f32;
        for step in 1..num_steps {
            let candidate_position = bounds_min + step as f32 * step_size;
            let cost = evaluate_sah(node, axis, candidate_position, tris, centroids, tri_indices);
            if cost < best_cost {
                best_position = candidate_position;
                best_axis = axis;
                best_cost = cost;
            }
        }
    }

    (best_axis, best_position, best_cost)
}

fn calculate_node_cost(node: &PulseBLASNode) -> f32 {
    let e = node.aabb_max - node.aabb_min;
    let area = e.x * e.y + e.y * e.z + e.z * e.x;
    node.tri_count as f32 * area
}

fn evaluate_sah(
    node: &PulseBLASNode,
    axis: usize,
    position: f32,
    tris: &Vec<PulseTriangle>,
    centroids: &Vec<Vec3>,
    tri_indices: &Vec<usize>,
) -> f32 {
    let mut box_a = AABB::default();
    let mut box_b = AABB::default();
    let mut a_count = 0;
    let mut b_count = 0;
    for i in 0..node.tri_count {
        let triangle = &tris[tri_indices[(node.first_tri + i) as usize]];
        let centroid = centroids[tri_indices[(node.first_tri + i) as usize]];
        if centroid[axis] < position {
            a_count += 1;
            box_a.grow_position(triangle.positions[0]);
            box_a.grow_position(triangle.positions[1]);
            box_a.grow_position(triangle.positions[2]);
        } else {
            b_count += 1;
            box_b.grow_position(triangle.positions[0]);
            box_b.grow_position(triangle.positions[1]);
            box_b.grow_position(triangle.positions[2]);
        }
    }
    let cost = a_count as f32 * box_a.area() + b_count as f32 * box_b.area();
    if cost > 0.0 {
        cost
    } else {
        1e32
    }
}

fn calculate_node_aabb(
    node: &mut PulseBLASNode,
    tris: &Vec<PulseTriangle>,
    tri_indices: &Vec<usize>,
) {
    node.aabb_min = Vec3::MAX;
    node.aabb_max = Vec3::MIN;
    for i in node.first_tri..(node.first_tri + node.tri_count) {
        let tri_index = tri_indices[i as usize];

        node.aabb_min = node.aabb_min.min(tris[tri_index].positions[0]);
        node.aabb_min = node.aabb_min.min(tris[tri_index].positions[1]);
        node.aabb_min = node.aabb_min.min(tris[tri_index].positions[2]);

        node.aabb_max = node.aabb_max.max(tris[tri_index].positions[0]);
        node.aabb_max = node.aabb_max.max(tris[tri_index].positions[1]);
        node.aabb_max = node.aabb_max.max(tris[tri_index].positions[2]);
    }
}

pub fn swap<T: Clone>(data: &mut [T], i0: usize, i1: usize) {
    // TODO: Error handling
    let val0 = data[i0].clone();
    data[i0] = data[i1].clone();
    data[i1] = val0;
}
