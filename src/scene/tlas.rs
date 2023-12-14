use super::*;
use bevy::{prelude::*, render::render_resource::ShaderType};
use std::time::Instant;

#[derive(Default, ShaderType, Clone, Debug)]
pub struct PulseTLASNode {
    pub aabb_min: Vec3,
    pub a_or_first_instance: u32,
    pub aabb_max: Vec3,
    pub instance_count: u32,
}

fn build_tlas(blas_nodes: &Vec<PulseBLASNode>, instances: &Vec<PulseMeshInstance>) {
    let mut centroids: Vec<Vec3> = vec![];
    for i in 0..instances.len() {
        centroids.push(
            instances[i]
                .transform
                .mul_vec4(Vec4::new(0.0, 0.0, 0.0, 1.0))
                .xyz(),
        );
    }

    let mut tlas_nodes: Vec<PulseTLASNode> = vec![];
    let mut root = PulseTLASNode::default();
    root.child_a_idx = 0;
    root.first_instance = 0;
    root.instance_count = instances.len() as u32;
    calculate_node_aabb(&mut root, instances, blas_nodes)
    tlas_nodes.push(root);

}

fn subdivide(
    node_idx: usize,
    nodes: &mut Vec<PulseTLASNode>,
    blas_nodes: &Vec<PulseBLASNode>,
    centroids: &Vec<Vec3>,
) {
    if nodes[node_idx].instance_count <= 1 {
        return;
    }
}

// Returns (axis, position, cost)
fn find_best_split_plane(
    node: &PulseTLASNode,
    blas_nodes: &Vec<PulseBLASNode>,
    centroids: &Vec<Vec3>,
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

fn calculate_node_aabb(
    node: &mut PulseTLASNode,
    instances: &Vec<PulseMeshInstance>,
    blas_nodes: &Vec<PulseBLASNode>,
) {
    for i in node.first_instance..(node.first_instance + node.instance_count) {
        let blas_root = &blas_nodes[instances[i as usize].mesh_index.node_offset as usize];
        // Blas min/max corners in world space.
        let blas_min = transform(instances[i as usize].transform, blas_root.aabb_min);
        let blas_max = transform(instances[i as usize].transform, blas_root.aabb_max);

        node.aabb_min = node.aabb_min.min(blas_min);
        node.aabb_max = node.aabb_max.max(blas_max);
    }
}

pub fn swap<T: Clone>(data: &mut [T], i0: usize, i1: usize) {
    // TODO: Error handling
    let val0 = data[i0].clone();
    data[i0] = data[i1].clone();
    data[i1] = val0;
}
