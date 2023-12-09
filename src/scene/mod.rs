use crate::utilities::*;
use bevy::{
    asset::load_internal_asset,
    prelude::*,
    render::{
        camera::{CameraRenderGraph, ExtractedCamera},
        extract_component::ExtractComponent,
        mesh::{GpuBufferInfo, GpuMesh, Indices, VertexAttributeValues},
        render_asset::{ExtractedAssets, RenderAssets},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::TextureCache,
        view::ViewUniforms,
        Extract, Render, RenderApp, RenderSet,
    },
    utils::{HashMap, HashSet},
};
use std::num::NonZeroU32;

pub mod bvh;
use bvh::*;

pub struct PulseScenePlugin;

impl Plugin for PulseScenePlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(
                ExtractSchedule,
                (extract_mesh_assets, extract_mesh_instances),
            )
            .add_systems(
                Render,
                (
                    prepare_extracted_mesh_assets,
                    prepare_mesh_data,
                    prepare_mesh_instances,
                )
                    .in_set(RenderSet::Prepare),
            )
            .add_systems(Render, queue_scene_bind_group.in_set(RenderSet::Queue));

        render_app
            .init_resource::<ExtractedMeshAssets>()
            .init_resource::<PulseMeshes>()
            .init_resource::<ExtractedMeshInstances>()
            .init_resource::<PulseMeshIndices>()
            .init_resource::<PulseMeshInstances>()
            .init_resource::<PulsePreparedMeshAssetData>();
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<PulseSceneBindGroup>()
            .init_resource::<PulseSceneBindGroupLayout>();
    }
}

#[derive(Resource, Default)]
pub struct ExtractedMeshAssets {
    pub new_or_modified: Vec<(AssetId<Mesh>, Mesh)>,
    pub removed: Vec<AssetId<Mesh>>,
}

fn extract_mesh_assets(
    mut mesh_asset_events: Extract<EventReader<AssetEvent<Mesh>>>,
    mesh_assets: Extract<Res<Assets<Mesh>>>,
    mut extracted: ResMut<ExtractedMeshAssets>,
) {
    let mut new_or_modified = Vec::new();
    let mut removed = Vec::new();
    for event in mesh_asset_events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                if let Some(mesh) = mesh_assets.get(*id) {
                    info!("Extracted mesh with id {:?}", id);
                    new_or_modified.push((id.clone(), mesh.clone()));
                }
            }
            AssetEvent::Removed { id } => {
                removed.push(id.clone());
            }
            AssetEvent::LoadedWithDependencies { .. } => {}
        }
    }

    extracted.new_or_modified = new_or_modified;
    extracted.removed = removed;
}

#[derive(Default, ShaderType, Clone, Debug)]
pub struct PulseTriangle {
    pub positions: [Vec3; 3],
    pub normals: [Vec3; 3],
    pub uvs: [Vec2; 3],
}

pub struct PulseMesh {
    pub triangles: Vec<PulseTriangle>,
    pub bvh: BVH,
}

#[derive(Resource, Default)]
pub struct PulseMeshes(pub HashMap<AssetId<Mesh>, PulseMesh>);

// Prepare triangle data and build bvh for new/modified mesh assets and remove when not used anymore.
fn prepare_extracted_mesh_assets(
    extracted: Res<ExtractedMeshAssets>,
    mut meshes: ResMut<PulseMeshes>,
) {
    for (id, mesh) in extracted.new_or_modified.iter() {
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(VertexAttributeValues::as_float3)
            .unwrap()
            .iter()
            .map(|p| Vec3::from_array(*p))
            .collect::<Vec<Vec3>>();
        let normals = mesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .and_then(VertexAttributeValues::as_float3)
            .unwrap()
            .iter()
            .map(|n| Vec3::from_array(*n))
            .collect::<Vec<Vec3>>();
        let uvs = mesh
            .attribute(Mesh::ATTRIBUTE_UV_0)
            .and_then(|attribute| match attribute {
                VertexAttributeValues::Float32x2(value) => Some(value),
                _ => None,
            })
            .unwrap()
            .iter()
            .map(|uv| Vec2::from_array(*uv))
            .collect::<Vec<Vec2>>();

        let indices: Vec<u32> = match mesh.indices() {
            Some(Indices::U16(..)) => {
                error!("Cannot load mesh that uses a u16 index buffer.");
                return;
            }
            Some(Indices::U32(values)) => values.clone(),
            None => {
                error!("Cannot load mesh with no index buffer.");
                return;
            }
        };
        let mut triangles = vec![];
        for i_0 in 0..(indices.len() / 3) {
            let i_0 = i_0 * 3;
            let v_0 = indices[i_0] as usize;
            let v_1 = indices[i_0 + 1] as usize;
            let v_2 = indices[i_0 + 2] as usize;
            triangles.push(PulseTriangle {
                positions: [positions[v_0], positions[v_1], positions[v_2]],
                normals: [normals[v_0], normals[v_1], normals[v_2]],
                uvs: [uvs[v_0], uvs[v_1], uvs[v_2]],
            })
        }

        let bvh = build_bvh(&triangles);
        meshes.0.insert(id.clone(), PulseMesh { triangles, bvh });
    }

    for id in extracted.removed.iter() {
        meshes.0.remove(id);
    }
}

#[derive(Resource, Default)]
pub struct PulsePreparedMeshAssetData {
    pub triangles: Vec<PulseTriangle>,
    pub indices: Vec<u32>,
    pub nodes: Vec<BVHNode>,
}

// Index into buffers in `PulsePreparedMeshAssetData`.
#[derive(ShaderType, Copy, Clone, Debug)]
pub struct PulseMeshIndex {
    pub triangle_offset: u32,
    // pub triangle_count: u32,
    pub index_offset: u32,
    // pub index_count: u32,
    pub node_offset: u32,
    // pub node_count: u32,
}

#[derive(Resource, Default)]
pub struct PulseMeshIndices(pub HashMap<AssetId<Mesh>, PulseMeshIndex>);

fn prepare_mesh_data(
    meshes: Res<PulseMeshes>,
    extracted: Res<ExtractedMeshAssets>,
    mut prepared_mesh_data: ResMut<PulsePreparedMeshAssetData>,
    mut mesh_indices: ResMut<PulseMeshIndices>,
) {
    // Abort if mesh data is the same as last frame's.
    if extracted.new_or_modified.len() == 0 && extracted.removed.len() == 0 {
        return;
    }

    for (id, mesh) in meshes.0.iter() {
        let triangle_offset = prepared_mesh_data.triangles.len() as u32;
        // let triangle_count = mesh.triangles.len() as u32;

        let index_offset = prepared_mesh_data.indices.len() as u32;
        // let index_count = mesh.bvh.tri_indices.len() as u32;

        let node_offset = prepared_mesh_data.nodes.len() as u32;
        // let node_count = mesh.bvh.nodes.len() as u32;

        mesh_indices.0.insert(
            id.clone(),
            PulseMeshIndex {
                triangle_offset,
                // triangle_count,
                index_offset,
                // index_count,
                node_offset,
                // node_count,
            },
        );

        prepared_mesh_data.triangles.extend(mesh.triangles.clone());
        prepared_mesh_data
            .indices
            .extend(mesh.bvh.tri_indices.clone());
        prepared_mesh_data.nodes.extend(mesh.bvh.nodes.clone());
    }
}

#[derive(Resource, Default)]
pub struct ExtractedMeshInstances(pub Vec<(Handle<Mesh>, GlobalTransform)>);

pub fn extract_mesh_instances(
    query: Extract<Query<(&Handle<Mesh>, &GlobalTransform)>>,
    mut extracted: ResMut<ExtractedMeshInstances>,
) {
    extracted.0 = query
        .iter()
        .map(|(handle, transform)| (handle.clone(), transform.clone()))
        .collect::<Vec<(Handle<Mesh>, GlobalTransform)>>();
}

#[derive(ShaderType, Copy, Clone, Debug)]
pub struct PulseMeshInstance {
    pub transform: Mat4,
    pub transform_inv: Mat4,
    pub mesh_index: PulseMeshIndex,
}

#[derive(Resource, Default, Debug)]
pub struct PulseMeshInstances(pub Vec<PulseMeshInstance>);

pub fn prepare_mesh_instances(
    extracted: Res<ExtractedMeshInstances>,
    mesh_indices: Res<PulseMeshIndices>,
    mut mesh_instances: ResMut<PulseMeshInstances>,
) {
    mesh_instances.0 = vec![];
    // TODO: Only update instances when added/removed
    for (handle, transform) in &extracted.0 {
        let Handle::Weak(id) = handle.clone_weak() else {
            continue;
        };
        let Some(mesh_index) = mesh_indices.0.get(&id) else {
            continue;
        };
        let transform = transform.compute_matrix();
        let transform_inv = transform.inverse();
        mesh_instances.0.push(PulseMeshInstance {
            transform,
            transform_inv,
            mesh_index: mesh_index.clone(),
        })
    }
}

#[derive(Resource)]
pub struct PulseSceneBindGroupLayout(pub BindGroupLayout);

impl FromWorld for PulseSceneBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        Self(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("pulse_scene_bind_group_layout"),
            entries: &[
                // Uniform
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Triangles
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Triangle indices
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Nodes
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Instances
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        }))
    }
}

#[derive(ShaderType, Default)]
pub struct PulseSceneUniform {
    pub instance_count: u32,
}

#[derive(Resource, Default)]
pub struct PulseSceneBindGroup(pub Option<BindGroup>);

// TODO: Don't run every frame
fn queue_scene_bind_group(
    mesh_data: Res<PulsePreparedMeshAssetData>,
    instances: Res<PulseMeshInstances>,
    mut bind_group: ResMut<PulseSceneBindGroup>,
    layout: Res<PulseSceneBindGroupLayout>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let uniform = PulseSceneUniform {
        instance_count: instances.0.len() as u32,
    };

    let uniform_buffer = create_uniform_buffer(
        uniform,
        Some("pulse_scene_uniform"),
        &render_device,
        &render_queue,
    );

    let triangle_buffer = create_storage_buffer(
        mesh_data.triangles.clone(),
        Some("pulse_triangle_buffer"),
        &render_device,
        &render_queue,
    );

    let triangle_index_buffer = create_storage_buffer(
        mesh_data.indices.clone(),
        Some("pulse_triangle_index_buffer"),
        &render_device,
        &render_queue,
    );

    let node_buffer = create_storage_buffer(
        mesh_data.nodes.clone(),
        Some("pulse_node_buffer"),
        &render_device,
        &render_queue,
    );

    let instance_buffer = create_storage_buffer(
        instances.0.clone(),
        Some("pulse_node_buffer"),
        &render_device,
        &render_queue,
    );

    // warn!("{:?}", instances.0.clone());

    bind_group.0 = Some(render_device.create_bind_group(
        Some("pulse_scene_bind_group"),
        &layout.0,
        &[
            BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 1,
                resource: triangle_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 2,
                resource: triangle_index_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 3,
                resource: node_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 4,
                resource: instance_buffer.binding().unwrap(),
            },
        ],
    ));
}
