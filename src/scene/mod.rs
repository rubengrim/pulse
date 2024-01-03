use crate::utilities::*;
use bevy::{
    asset::load_internal_asset,
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics, RegisterDiagnostic},
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        Extract, Render, RenderApp, RenderSet,
    },
    utils::HashMap,
};
use std::sync::Mutex;
use std::time::Instant;

pub mod blas;
use blas::*;
pub mod tlas;
use tlas::*;

pub const PULSE_SCENE_BINDINGS_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(187737725855836603431472235313437654946);

pub const PULSE_SCENE_TYPES_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(187731725155836403431412235313437654946);

pub const TLAS_BUILD_TIME: DiagnosticId =
    DiagnosticId::from_u128(178146834822086073791974408528866909483);

pub const INSTANCE_PREPARE_TIME: DiagnosticId =
    DiagnosticId::from_u128(178146834822086073791974408528866909483);

pub struct PulseScenePlugin;

impl Plugin for PulseScenePlugin {
    fn build(&self, app: &mut App) {
        // app.init_resource::<AABBsToDraw>()
        //     .add_systems(Update, draw_aabbs);
        load_internal_asset!(
            app,
            PULSE_SCENE_BINDINGS_SHADER_HANDLE,
            "bindings.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            PULSE_SCENE_TYPES_SHADER_HANDLE,
            "types.wgsl",
            Shader::from_wgsl
        );

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(
                ExtractSchedule,
                (
                    extract_material_assets,
                    extract_mesh_assets,
                    extract_mesh_material_instances,
                    // send_aabbs_to_app_world,
                ),
            )
            .add_systems(
                Render,
                (
                    prepare_extracted_mesh_assets,
                    prepare_mesh_data,
                    prepare_mesh_instances,
                    prepare_extracted_material_assets,
                    prepare_material_data,
                )
                    .in_set(RenderSet::Prepare),
            )
            .add_systems(Render, queue_scene_bind_group.in_set(RenderSet::Queue));

        render_app
            .register_diagnostic(
                Diagnostic::new(TLAS_BUILD_TIME, "tlas_build_time", 20)
                    .with_suffix("ms")
                    .with_smoothing_factor(1.0),
            )
            .register_diagnostic(
                Diagnostic::new(INSTANCE_PREPARE_TIME, "instance_prepare_time", 20)
                    .with_suffix("ms")
                    .with_smoothing_factor(1.0),
            );

        render_app
            .init_resource::<ExtractedMeshAssets>()
            .init_resource::<PulseMeshes>()
            .init_resource::<ExtractedMeshMaterialInstances>()
            .init_resource::<PulseMeshIndices>()
            .init_resource::<PulseMeshInstances>()
            .init_resource::<PulsePreparedMeshAssetData>()
            .init_resource::<PulseSceneTLAS>()
            .init_resource::<ExtractedMaterialAssets>()
            .init_resource::<PulseMaterials>()
            .init_resource::<PulseMaterialIndices>()
            .init_resource::<PulsePreparedMaterialAssetData>();
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<PulseSceneBindGroup>()
            .init_resource::<PulseSceneBindGroupLayout>();
    }
}

#[derive(Resource, Default)]
pub struct AABBsToDraw(pub Mutex<Vec<(Vec3, Vec3)>>);

// pub fn send_aabbs_to_app_world(
//     main_world: ResMut<MainWorld>,
//     instances: Res<ExtractedMeshInstances>,
//     meshes: Res<PulseMeshes>,
//     tlas: Res<PulseSceneTLAS>,
// ) {
//     let mut aabbs = main_world.resource::<AABBsToDraw>().0.lock().unwrap();
//     *aabbs = vec![];
//     // for (handle, transform) in instances.0.iter() {
//     //     let Handle::Weak(id) = handle.clone_weak() else {
//     //         continue;
//     //     };
//     //     let transform_mat = transform.compute_matrix();
//     //     let Some(mesh) = meshes.0.get(&id) else {
//     //         continue;
//     //     };
//     //     for node in &mesh.bvh.nodes {
//     //         if node.tri_count > 0 {
//     //             // warn!("tri_count: {}", node.tri_count);
//     //             let min = transform_pos(transform_mat, node.aabb_min);
//     //             let max = transform_pos(transform_mat, node.aabb_max);
//     //             aabbs.push((min, max));
//     //         }
//     //     }
//     // }
//     for node in &tlas.0.nodes {
//         aabbs.push((node.aabb_min, node.aabb_max));
//     }
// }

// fn draw_aabbs(aabbs: Res<AABBsToDraw>, mut gizmos: Gizmos) {
//     let aabbs = &*aabbs.0.lock().unwrap();
//     for (min, max) in aabbs.iter() {
//         let e = *max - *min;
//         gizmos.cuboid(
//             Transform::from_translation(*min + 0.5 * e).with_scale(Vec3::new(e.x, e.y, e.z)),
//             Color::CYAN,
//         );
//     }
// }

#[derive(Resource, Default)]
pub struct ExtractedMaterialAssets {
    pub new_or_modified: Vec<(AssetId<StandardMaterial>, StandardMaterial)>,
    pub removed: Vec<AssetId<StandardMaterial>>,
}

impl ExtractedMaterialAssets {
    pub fn empty(&self) -> bool {
        self.new_or_modified.len() == 0 && self.new_or_modified.len() == 0
    }
}

fn extract_material_assets(
    mut material_asset_events: Extract<EventReader<AssetEvent<StandardMaterial>>>,
    material_assets: Extract<Res<Assets<StandardMaterial>>>,
    mut extracted: ResMut<ExtractedMaterialAssets>,
) {
    let mut new_or_modified = Vec::new();
    let mut removed = Vec::new();
    for event in material_asset_events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                if let Some(material) = material_assets.get(*id) {
                    info!("Extracted material with id {:?}", id);
                    new_or_modified.push((id.clone(), material.clone()));
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

#[derive(ShaderType, Clone)]
pub struct PulseMaterial {
    pub base_color: Vec4,
    pub emissive: Vec4,
    pub perceptual_roughness: f32,
    pub reflectance: f32,
    pub metallic: f32,
}

#[derive(Resource, Default, Deref, DerefMut)]
struct PulseMaterials(pub HashMap<AssetId<StandardMaterial>, PulseMaterial>);

fn prepare_extracted_material_assets(
    extracted: Res<ExtractedMaterialAssets>,
    mut materials: ResMut<PulseMaterials>,
) {
    for (id, material) in extracted.new_or_modified.iter() {
        let pulse_material = PulseMaterial {
            base_color: material.base_color.into(),
            emissive: material.emissive.into(),
            perceptual_roughness: material.perceptual_roughness,
            reflectance: material.reflectance,
            metallic: material.metallic,
        };

        materials.insert(id.clone(), pulse_material);
    }

    for id in extracted.removed.iter() {
        materials.remove(id);
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
struct PulsePreparedMaterialAssetData(pub Vec<PulseMaterial>);

#[derive(Resource, Default, Deref, DerefMut)]
struct PulseMaterialIndices(pub HashMap<AssetId<StandardMaterial>, u32>);

fn prepare_material_data(
    materials: Res<PulseMaterials>,
    extracted: Res<ExtractedMaterialAssets>,
    mut material_data: ResMut<PulsePreparedMaterialAssetData>,
    mut material_indices: ResMut<PulseMaterialIndices>,
) {
    // Abort if material data is the same as last frame's.
    if extracted.empty() {
        return;
    }

    *material_data = PulsePreparedMaterialAssetData::default();
    *material_indices = PulseMaterialIndices::default();
    for (id, material) in materials.0.iter() {
        let index = material_data.len() as u32;
        material_indices.insert(id.clone(), index);
        material_data.push(material.clone());
    }
}

#[derive(Resource, Default)]
struct ExtractedMeshAssets {
    pub new_or_modified: Vec<(AssetId<Mesh>, Mesh)>,
    pub removed: Vec<AssetId<Mesh>>,
}

impl ExtractedMeshAssets {
    pub fn empty(&self) -> bool {
        self.new_or_modified.len() == 0 && self.new_or_modified.len() == 0
    }
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
pub struct PulseTriangleData {
    pub normals: [Vec3; 3],
    pub uvs: [Vec2; 3],
}

#[derive(Default, ShaderType, Clone, Debug)]
pub struct PulsePrimitive {
    pub positions: [Vec3; 3],
}

pub struct PulseMesh {
    pub primitives: Vec<PulsePrimitive>,
    pub triangle_data: Vec<PulseTriangleData>,
    pub bvh: Blas,
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
            Some(Indices::U16(values)) => values.iter().map(|v| *v as u32).collect::<Vec<u32>>(),
            Some(Indices::U32(values)) => values.clone(),
            None => {
                error!("Cannot load mesh with no index buffer.");
                return;
            }
        };
        let mut triangle_data = vec![];
        let mut primitives = vec![];
        for i_0 in 0..(indices.len() / 3) {
            let i_0 = i_0 * 3;
            let v_0 = indices[i_0] as usize;
            let v_1 = indices[i_0 + 1] as usize;
            let v_2 = indices[i_0 + 2] as usize;
            primitives.push(PulsePrimitive {
                positions: [positions[v_0], positions[v_1], positions[v_2]],
            });
            triangle_data.push(PulseTriangleData {
                normals: [normals[v_0], normals[v_1], normals[v_2]],
                uvs: [uvs[v_0], uvs[v_1], uvs[v_2]],
            })
        }

        let blas_time_begin = Instant::now();
        let bvh = build_blas(&primitives);
        info!(
            "Built BLAS for mesh id:{:?} with triangle count {:?} in {:.3?}",
            id,
            primitives.len(),
            blas_time_begin.elapsed(),
        );

        meshes.0.insert(
            id.clone(),
            PulseMesh {
                primitives,
                triangle_data,
                bvh,
            },
        );
    }

    for id in extracted.removed.iter() {
        meshes.0.remove(id);
    }
}

#[derive(Resource, Default)]
pub struct PulsePreparedMeshAssetData {
    pub primitives: Vec<PulsePrimitive>,
    pub triangle_data: Vec<PulseTriangleData>,
    pub indices: Vec<u32>,
    pub nodes: Vec<PulseBLASNode>,
    pub materials: Vec<PulseMaterial>,
}

// Index into buffers in `PulsePreparedMeshAssetData`.
#[derive(ShaderType, Copy, Clone, Debug)]
pub struct PulseMeshIndex {
    pub triangle_offset: u32,
    pub index_offset: u32,
    pub node_offset: u32,
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
    if extracted.empty() {
        return;
    }

    *prepared_mesh_data = PulsePreparedMeshAssetData::default();
    *mesh_indices = PulseMeshIndices::default();
    for (id, mesh) in meshes.0.iter() {
        let triangle_offset = prepared_mesh_data.primitives.len() as u32;
        let index_offset = prepared_mesh_data.indices.len() as u32;
        let node_offset = prepared_mesh_data.nodes.len() as u32;

        mesh_indices.0.insert(
            id.clone(),
            PulseMeshIndex {
                triangle_offset,
                index_offset,
                node_offset,
            },
        );

        prepared_mesh_data
            .primitives
            .extend(mesh.primitives.clone());
        prepared_mesh_data
            .triangle_data
            .extend(mesh.triangle_data.clone());
        prepared_mesh_data
            .indices
            .extend(mesh.bvh.tri_indices.clone());
        prepared_mesh_data.nodes.extend(mesh.bvh.nodes.clone());
    }
}

#[derive(Resource, Default)]
pub struct ExtractedMeshMaterialInstances(
    pub Vec<(Handle<Mesh>, Handle<StandardMaterial>, GlobalTransform)>,
);

pub fn extract_mesh_material_instances(
    query: Extract<Query<(&Handle<Mesh>, &Handle<StandardMaterial>, &GlobalTransform)>>,
    mut extracted: ResMut<ExtractedMeshMaterialInstances>,
) {
    extracted.0 = query
        .iter()
        .map(|(mesh, material, transform)| (mesh.clone(), material.clone(), transform.clone()))
        .collect::<Vec<(Handle<Mesh>, Handle<StandardMaterial>, GlobalTransform)>>();
}

#[derive(ShaderType, Copy, Clone, Debug)]
pub struct PulseMeshInstance {
    pub transform: Mat4,
    pub transform_inv: Mat4,
    pub mesh_index: PulseMeshIndex,
    pub material_index: u32,
}

pub struct PulsePrimitiveMeshInstance {
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    pub center: Vec3,
}

#[derive(Resource, Default)]
pub struct PulseSceneTLAS(pub PulseTLAS);

#[derive(Resource, Default, Debug)]
pub struct PulseMeshInstances(pub Vec<PulseMeshInstance>);

fn prepare_mesh_instances(
    extracted: Res<ExtractedMeshMaterialInstances>,
    mesh_indices: Res<PulseMeshIndices>,
    mesh_data: Res<PulsePreparedMeshAssetData>,
    material_indices: Res<PulseMaterialIndices>,
    mut mesh_instances: ResMut<PulseMeshInstances>,
    mut tlas: ResMut<PulseSceneTLAS>,
    mut diagnostics: Diagnostics,
) {
    mesh_instances.0 = vec![];
    let mut instance_primitives: Vec<PulsePrimitiveMeshInstance> = vec![]; // Used for TLAS creation.

    let instance_prepare_start_time = Instant::now();
    for (mesh_handle, material_handle, transform) in &extracted.0 {
        let (Handle::Weak(mesh_id), Handle::Weak(material_id)) =
            (mesh_handle.clone_weak(), material_handle.clone_weak())
        else {
            continue;
        };
        let (Some(mesh_index), Some(material_index)) = (
            mesh_indices.0.get(&mesh_id),
            material_indices.0.get(&material_id),
        ) else {
            continue;
        };
        let transform = transform.compute_matrix();
        let transform_inv = transform.inverse();
        mesh_instances.0.push(PulseMeshInstance {
            transform,
            transform_inv,
            mesh_index: mesh_index.clone(),
            material_index: *material_index,
        });

        let root_node = &mesh_data.nodes[mesh_index.node_offset as usize];
        let b_min = root_node.aabb_min;
        let b_max = root_node.aabb_max;
        // Calculate world space bounds.
        let mut b_min_world = Vec3::MAX;
        let mut b_max_world = Vec3::MIN;

        let corner_1 = transform_position(Vec3::new(b_min.x, b_min.y, b_min.z), transform);
        let corner_2 = transform_position(Vec3::new(b_max.x, b_min.y, b_min.z), transform);
        let corner_3 = transform_position(Vec3::new(b_min.x, b_max.y, b_min.z), transform);
        let corner_4 = transform_position(Vec3::new(b_min.x, b_min.y, b_max.z), transform);
        let corner_5 = transform_position(Vec3::new(b_max.x, b_max.y, b_min.z), transform);
        let corner_6 = transform_position(Vec3::new(b_min.x, b_max.y, b_max.z), transform);
        let corner_7 = transform_position(Vec3::new(b_max.x, b_min.y, b_max.z), transform);
        let corner_8 = transform_position(Vec3::new(b_max.x, b_max.y, b_max.z), transform);

        b_min_world = b_min_world.min(corner_1);
        b_min_world = b_min_world.min(corner_2);
        b_min_world = b_min_world.min(corner_3);
        b_min_world = b_min_world.min(corner_4);
        b_min_world = b_min_world.min(corner_5);
        b_min_world = b_min_world.min(corner_6);
        b_min_world = b_min_world.min(corner_7);
        b_min_world = b_min_world.min(corner_8);

        b_max_world = b_max_world.max(corner_1);
        b_max_world = b_max_world.max(corner_2);
        b_max_world = b_max_world.max(corner_3);
        b_max_world = b_max_world.max(corner_4);
        b_max_world = b_max_world.max(corner_5);
        b_max_world = b_max_world.max(corner_6);
        b_max_world = b_max_world.max(corner_7);
        b_max_world = b_max_world.max(corner_8);

        let center = b_min_world + 0.5 * (b_max_world - b_min_world);

        // for i in 0..8 {
        //     let corner_x = if i & 1 == 1 { b_max.x } else { b_min.x };
        //     let corner_y = if i & 2 == 1 { b_max.y } else { b_min.y };
        //     let corner_z = if i & 4 == 1 { b_max.z } else { b_min.z };
        //     let corner_world =
        //         transform_position(transform, Vec3::new(corner_x, corner_y, corner_z));
        //     b_min_world = b_min_world.min(corner_world);
        //     b_max_world = b_max_world.max(corner_world);
        // }

        // let root_node = &mesh_data.nodes[mesh_index.node_offset as usize];
        // let center = root_node.aabb_min + 0.5 * (root_node.aabb_max - root_node.aabb_min);
        // let center = transform_position(transform, center);
        // let bounds_min = transform_position(transform, root_node.aabb_min);
        // let bounds_max = transform_position(transform, root_node.aabb_max);

        instance_primitives.push(PulsePrimitiveMeshInstance {
            bounds_min: b_min_world,
            bounds_max: b_max_world,
            center,
        });
    }
    diagnostics.add_measurement(INSTANCE_PREPARE_TIME, || {
        instance_prepare_start_time.elapsed().as_secs_f64() * 1000.0
    });

    let tlas_time_begin = Instant::now();
    tlas.0 = build_tlas(&instance_primitives);
    diagnostics.add_measurement(TLAS_BUILD_TIME, || {
        tlas_time_begin.elapsed().as_secs_f64() * 1000.0
    });
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
                // Primitives
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
                // Triangle data
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
                // Triangle indices
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
                // BLAS nodes
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
                // TLAS nodes
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Instance indices
                BindGroupLayoutEntry {
                    binding: 6,
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
                    binding: 7,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Materials
                BindGroupLayoutEntry {
                    binding: 8,
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
    material_data: Res<PulsePreparedMaterialAssetData>,
    instances: Res<PulseMeshInstances>,
    tlas: Res<PulseSceneTLAS>,
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

    let primitive_buffer = create_storage_buffer(
        mesh_data.primitives.clone(),
        Some("pulse_primitive_buffer"),
        &render_device,
        &render_queue,
    );

    let triangle_data_buffer = create_storage_buffer(
        mesh_data.triangle_data.clone(),
        Some("pulse_triangle_data_buffer"),
        &render_device,
        &render_queue,
    );

    let triangle_index_buffer = create_storage_buffer(
        mesh_data.indices.clone(),
        Some("pulse_triangle_index_buffer"),
        &render_device,
        &render_queue,
    );

    let blas_node_buffer = create_storage_buffer(
        mesh_data.nodes.clone(),
        Some("pulse_blas_node_buffer"),
        &render_device,
        &render_queue,
    );

    let tlas_node_buffer = create_storage_buffer(
        tlas.0.nodes.clone(),
        Some("pulse_tlas_node_buffer"),
        &render_device,
        &render_queue,
    );

    let instance_index_buffer = create_storage_buffer(
        tlas.0.instance_indices.clone(),
        Some("pulse_instance_index_buffer"),
        &render_device,
        &render_queue,
    );

    let instance_buffer = create_storage_buffer(
        instances.0.clone(),
        Some("pulse_instance_buffer"),
        &render_device,
        &render_queue,
    );

    let material_buffer = create_storage_buffer(
        material_data.0.clone(),
        Some("pulse_material_buffer"),
        &render_device,
        &render_queue,
    );

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
                resource: primitive_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 2,
                resource: triangle_data_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 3,
                resource: triangle_index_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 4,
                resource: blas_node_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 5,
                resource: tlas_node_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 6,
                resource: instance_index_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 7,
                resource: instance_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 8,
                resource: material_buffer.binding().unwrap(),
            },
        ],
    ));
}
