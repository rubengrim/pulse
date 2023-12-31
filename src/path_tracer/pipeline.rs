use crate::scene::PulseSceneBindGroupLayout;

use super::{PulsePathTracerUniform, PulseRenderTarget, PULSE_PATH_TRACER_SHADER_HANDLE};
use bevy::{
    prelude::*,
    render::{
        camera::ExtractedCamera, render_resource::*, renderer::RenderDevice, view::ViewUniform,
    },
};

#[derive(Resource)]
pub struct PulsePathTracerPipeline {
    pub scene_layout: BindGroupLayout,
    pub view_layout: BindGroupLayout,
}

impl FromWorld for PulsePathTracerPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let view_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                // View uniforms
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(ViewUniform::min_size()),
                    },
                    count: None,
                },
                // Output texture view
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: PulseRenderTarget::TEXTURE_FORMAT,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Path tracer uniform
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(PulsePathTracerUniform::min_size()),
                    },
                    count: None,
                },
            ],
        });

        let scene_layout = world.resource::<PulseSceneBindGroupLayout>().0.clone();

        Self {
            scene_layout,
            view_layout,
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PulsePathTracerPipelineKey;

impl SpecializedComputePipeline for PulsePathTracerPipeline {
    type Key = PulsePathTracerPipelineKey;

    fn specialize(&self, _key: Self::Key) -> ComputePipelineDescriptor {
        ComputePipelineDescriptor {
            label: Some("pulse_path_tracer_pipeline".into()),
            layout: vec![self.scene_layout.clone(), self.view_layout.clone()],
            push_constant_ranges: vec![],
            shader: PULSE_PATH_TRACER_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "path_trace".into(),
        }
    }
}

#[derive(Component)]
pub struct PulsePathTracerPipelineId(pub CachedComputePipelineId);

pub fn prepare_pipelines(
    views: Query<Entity, With<ExtractedCamera>>,
    mut commands: Commands,
    mut compute_pipelines: ResMut<SpecializedComputePipelines<PulsePathTracerPipeline>>,
    compute_pipeline: Res<PulsePathTracerPipeline>,
    cache: Res<PipelineCache>,
) {
    for view_entity in &views {
        let compute_pipeline_id =
            compute_pipelines.specialize(&cache, &compute_pipeline, PulsePathTracerPipelineKey);
        commands
            .entity(view_entity)
            .insert(PulsePathTracerPipelineId(compute_pipeline_id));
    }
}
