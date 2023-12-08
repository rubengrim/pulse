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
            .add_systems(ExtractSchedule, extract_mesh_assets)
            .add_systems(Render, prepare_mesh_assets.in_set(RenderSet::Prepare))
            .add_systems(Render, queue_scene_bind_group.in_set(RenderSet::Queue));

        render_app
            .init_resource::<ExtractedMeshAssets>()
            .init_resource::<PulseMeshes>();
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

#[derive(ShaderType)]
pub struct PulseMeshIndex {
    pub triangle_start: u32,
    pub triangle_count: u32,
    pub node_start: u32,
    pub node_end: u32,
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

fn prepare_mesh_assets(extracted: Res<ExtractedMeshAssets>, mut meshes: ResMut<PulseMeshes>) {
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

// #[derive(Resource, Default)]
// pub struct ExtractedMeshInstances(pub Vec<(Handle<Mesh>, GlobalTransform)>);

// pub fn extract_mesh_instances(
//     query: Extract<Query<(&Handle<Mesh>, &GlobalTransform)>>,
//     mut extracted: ResMut<ExtractedMeshInstances>,
// ) {
//     extracted.0 = query
//         .iter()
//         .map(|(handle, transform)| (handle.clone(), transform.clone()))
//         .collect::<Vec<(Handle<Mesh>, GlobalTransform)>>();
// }

// // pub struct MeshInstances()

// pub fn prepare_mesh_instances(instances: Res<ExtractedMeshInstances>, meshes: Res<PulseMeshes>) {
//     // Assume only one instance for now
//     let instance = instances.0[0];
//     let Handle::Weak(id) = instance.0.clone_weak();
//     let mesh = meshes.0.get(&id).unwrap();
// }

#[derive(Resource)]
pub struct PulseSceneBindGroupLayout(pub BindGroupLayout);

impl FromWorld for PulseSceneBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        Self(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("pulse_scene_bind_group_layout"),
            entries: &[
                // Triangles
                BindGroupLayoutEntry {
                    binding: 0,
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
                    binding: 1,
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
                    binding: 2,
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

#[derive(Resource, Default)]
pub struct PulseSceneBindGroup(pub Option<BindGroup>);

fn queue_scene_bind_group(
    meshes: Res<PulseMeshes>,
    mut bind_group: ResMut<PulseSceneBindGroup>,
    layout: Res<PulseSceneBindGroupLayout>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    // Allt detta är väldigt temporärt

    let mut triangles = vec![];
    let mut nodes = vec![];
    let mut tri_indices = vec![];
    for mesh in meshes.0.values() {
        triangles = mesh.triangles.clone();
        nodes = mesh.bvh.nodes.clone();
        tri_indices = mesh.bvh.tri_indices.clone();
    }

    let triangle_buffer = create_storage_buffer(
        triangles,
        Some("pulse_triangle_buffer"),
        &render_device,
        &render_queue,
    );

    let triangle_index_buffer = create_storage_buffer(
        tri_indices,
        Some("pulse_triangle_index_buffer"),
        &render_device,
        &render_queue,
    );

    let node_buffer = create_storage_buffer(
        nodes,
        Some("pulse_node_buffer"),
        &render_device,
        &render_queue,
    );

    bind_group.0 = Some(render_device.create_bind_group(
        Some("pulse_scene_bind_group"),
        &layout.0,
        &[
            BindGroupEntry {
                binding: 0,
                resource: triangle_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 1,
                resource: triangle_index_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 2,
                resource: node_buffer.binding().unwrap(),
            },
        ],
    ));
}
