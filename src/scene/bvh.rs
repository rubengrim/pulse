use super::PulseTriangle;
use bevy::{prelude::*, render::render_resource::ShaderType};

#[derive(Default, ShaderType, Clone, Debug)]
pub struct BVHNode {
    pub aabb_min: Vec3,
    pub aabb_max: Vec3,
    // child_b_idx is always child_a_idx + 1 so don't store it.
    pub child_a_idx: u32,
    // Index for tri_indices, not directly for the tri data.
    pub first_primitive: u32,
    pub primitive_count: u32,
}

pub struct BVH {
    pub nodes: Vec<BVHNode>,
    pub tri_indices: Vec<u32>,
}

pub fn build_bvh(tris: &Vec<PulseTriangle>) -> BVH {
    let mut tri_indices: Vec<usize> = vec![];
    // TODO: Use AABB centers instead of triangle centroids.
    let mut centroids: Vec<Vec3> = vec![];
    for i in 0..tris.len() {
        centroids
            .push((tris[i].positions[0] + tris[i].positions[1] + tris[i].positions[2]) * 0.3333);
        tri_indices.push(i);
    }

    let mut nodes: Vec<BVHNode> = vec![];
    let mut root = BVHNode::default();
    root.child_a_idx = 0;
    root.first_primitive = 0;
    root.primitive_count = tri_indices.len() as u32;
    calculate_node_aabb(&mut root, tris, &tri_indices);
    nodes.push(root);

    subdivide(0, &mut nodes, tris, &centroids, &mut tri_indices);

    // Ugly temporary fix should already use u32
    let tri_indices = tri_indices.iter().map(|i| *i as u32).collect::<Vec<u32>>();

    warn!("{:?}", nodes);

    BVH { nodes, tri_indices }
}

pub fn subdivide(
    node_idx: usize,
    nodes: &mut Vec<BVHNode>,
    tris: &Vec<PulseTriangle>,
    centroids: &Vec<Vec3>,
    tri_indices: &mut Vec<usize>,
) {
    if nodes[node_idx].primitive_count <= 4 {
        return;
    }

    let extent = nodes[node_idx].aabb_max - nodes[node_idx].aabb_min;

    // Find longest axis.
    let mut axis = 0;
    if extent.y > extent.x {
        axis = 1;
    }
    if extent.z > extent[axis] {
        axis = 2;
    }

    let split_position = nodes[node_idx].aabb_min[axis] + 0.5 * extent[axis];

    let mut i = nodes[node_idx].first_primitive;
    let mut j = i + nodes[node_idx].primitive_count - 1;
    while i <= j {
        if centroids[tri_indices[i as usize]][axis] < split_position {
            i += 1;
        } else {
            swap(tri_indices, i as usize, j as usize);
            j -= 1;
        }
    }

    let a_count = i - nodes[node_idx].first_primitive;
    // Don't split the nodes[node_idx] if either one of it's children contain no primitives.
    if a_count == 0 || a_count == nodes[node_idx].primitive_count {
        return;
    }

    let node_count = nodes.len() as u32;

    let mut child_a = BVHNode::default();
    child_a.first_primitive = nodes[node_idx].first_primitive;
    child_a.primitive_count = a_count;
    calculate_node_aabb(&mut child_a, tris, tri_indices);
    nodes.push(child_a);

    let mut child_b = BVHNode::default();
    child_b.first_primitive = i;
    child_b.primitive_count = nodes[node_idx].primitive_count - a_count;
    calculate_node_aabb(&mut child_b, tris, tri_indices);
    nodes.push(child_b);

    nodes[node_idx].child_a_idx = node_count;
    // Parent nodes[node_idx] is not a leaf, so set prim count to 0.
    nodes[node_idx].primitive_count = 0;

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

fn calculate_node_aabb(node: &mut BVHNode, tris: &Vec<PulseTriangle>, tri_indices: &Vec<usize>) {
    node.aabb_min = Vec3::MAX;
    node.aabb_max = Vec3::MIN;
    for i in node.first_primitive..(node.first_primitive + node.primitive_count) {
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
