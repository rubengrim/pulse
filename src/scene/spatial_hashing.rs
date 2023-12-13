use super::PulseTriangle;
use bevy::{prelude::*, render::render_resource::ShaderType};
use std::f32::*;

#[derive(Default, Clone, ShaderType)]
pub struct PulseGridCell {
    pub first_tri: u32,
    pub tri_count: u32,
}

pub struct PulseGrid {
    pub cells: Vec<PulseGridCell>,
    pub indices: Vec<u32>,
    pub resolution_x: u32,
    pub resolution_y: u32,
    pub resolution_z: u32,
}

pub fn build_grid(tris: &Vec<PulseTriangle>) -> PulseGrid {
    let (b_min, b_max) = calculate_bounds(tris);
    let b_extent = b_max - b_min;
    let b_volume = b_extent.x * b_extent.y * b_extent.z;

    let density_factor = 5.0;
    let f = (density_factor * tris.len() as f32 / b_volume).powf(0.333);
    let subdivisions_x = (b_extent.x * f) as u32;
    let subdivisions_y = (b_extent.y * f) as u32;
    let subdivisions_z = (b_extent.x * f) as u32;

    // Temp override
    let resolution_x = 10;
    let resolution_y = 10;
    let resolution_z = 10;

    let cell_size_x = b_extent.x / resolution_x as f32;
    let cell_size_y = b_extent.y / resolution_y as f32;
    let cell_size_z = b_extent.z / resolution_z as f32;

    let num_cells = resolution_x * resolution_y * resolution_z;
    let mut cells: Vec<PulseGridCell> = vec![PulseGridCell::default(); num_cells];
    let mut indices: Vec<u32> = vec![];
    for x in 0..resolution_x {
        for y in 0..resolution_y {
            for z in 0..resolution_z {
                for i in 0..tris.len() {
                    let b_min = Vec3::new(
                        x as f32 * cell_size_x,
                        y as f32 * cell_size_y,
                        z as f32 * cell_size_z,
                    );
                    let b_max = Vec3::new(
                        (x + 1) as f32 * cell_size_x,
                        (y + 1) as f32 * cell_size_y,
                        (z + 1) as f32 * cell_size_z,
                    );
                    let first_tri = indices.len();
                    if triangle_aabb_intersect(tris[i].clone(), b_min, b_max) {
                        indices.push(i as u32);
                    }
                    let last_tri = indices.len();

                    // Calculate cell index.
                    let cell_index = x + resolution_x * (y + resolution_y * z);
                    cells[cell_index] = PulseGridCell {
                        first_tri: first_tri as u32,
                        tri_count: (last_tri - first_tri) as u32,
                    };
                }
            }
        }
    }

    PulseGrid {
        cells,
        indices,
        resolution_x: resolution_x as u32,
        resolution_y: resolution_y as u32,
        resolution_z: resolution_z as u32,
    }
}

fn calculate_bounds(tris: &Vec<PulseTriangle>) -> (Vec3, Vec3) {
    let mut min = Vec3::MAX;
    let mut max = Vec3::MIN;
    for tri in tris.iter() {
        min = min.min(tri.positions[0]);
        min = min.min(tri.positions[1]);
        min = min.min(tri.positions[2]);

        max = max.max(tri.positions[0]);
        max = max.max(tri.positions[1]);
        max = max.max(tri.positions[2]);
    }

    (min, max)
}

fn triangle_aabb_intersect(t: PulseTriangle, b_min: Vec3, b_max: Vec3) -> bool {
    // https://gdbooks.gitbooks.io/3dcollisions/content/Chapter4/aabb-triangle.html
    let mut v0 = t.positions[0];
    let mut v1 = t.positions[1];
    let mut v2 = t.positions[2];

    let e = b_max - b_min;
    let c = b_min + 0.5 * e;

    v0 -= c;
    v1 -= c;
    v2 -= c;

    let f0 = v1 - v0;
    let f1 = v2 - v1;
    let f2 = v0 - v2;

    // Face normals
    let u0 = Vec3::new(1.0, 0.0, 0.0);
    let u1 = Vec3::new(0.0, 1.0, 0.0);
    let u2 = Vec3::new(0.0, 0.0, 1.0);

    let mut axes: Vec<Vec3> = vec![];
    axes.push(u0.cross(f0));
    axes.push(u0.cross(f1));
    axes.push(u0.cross(f2));
    axes.push(u1.cross(f0));
    axes.push(u1.cross(f1));
    axes.push(u1.cross(f2));
    axes.push(u2.cross(f0));
    axes.push(u2.cross(f1));
    axes.push(u2.cross(f2));

    // AABB faces
    axes.push(u0);
    axes.push(u1);
    axes.push(u2);

    // Triangle face
    axes.push(f0.cross(f1));

    for axis in axes.iter() {
        let p0 = v0.dot(*axis);
        let p1 = v1.dot(*axis);
        let p2 = v2.dot(*axis);
        let r = e.x * u0.dot(*axis).abs() + e.y * u1.dot(*axis).abs() + e.z * u2.dot(*axis).abs();
        if f32::max(
            -f32::max(f32::max(p0, p1), p2),
            f32::min(f32::min(p0, p1), p2),
        ) > r
        {
            return false;
        }
    }

    true
}
