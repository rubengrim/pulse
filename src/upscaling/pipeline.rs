use crate::create_render_target_layout;

use super::PULSE_UPSCALING_SHADER_HANDLE;
use bevy::{
    prelude::*,
    render::{camera::ExtractedCamera, render_resource::*, renderer::RenderDevice},
};

#[derive(Resource)]
pub struct PulseUpscalingPipeline {
    pub render_target_layout: BindGroupLayout,
}

impl FromWorld for PulseUpscalingPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        Self {
            render_target_layout: create_render_target_layout(device),
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PulseUpscalingPipelineKey;

impl SpecializedRenderPipeline for PulseUpscalingPipeline {
    type Key = PulseUpscalingPipelineKey;

    fn specialize(&self, _key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("pulse_upscaling_pipeline".into()),
            layout: vec![self.render_target_layout.clone()],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: PULSE_UPSCALING_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "upscaling_vertex_shader".into(),
                buffers: vec![],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader: PULSE_UPSCALING_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "upscaling_fragment_shader".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Bgra8UnormSrgb,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
        }
    }
}

#[derive(Component)]
pub struct PulseUpscalingPipelineId(pub CachedRenderPipelineId);

pub fn prepare_upscaling_pipelines(
    views: Query<Entity, With<ExtractedCamera>>,
    mut commands: Commands,
    mut upscaling_pipelines: ResMut<SpecializedRenderPipelines<PulseUpscalingPipeline>>,
    upscaling_pipeline: Res<PulseUpscalingPipeline>,
    cache: Res<PipelineCache>,
) {
    for view_entity in &views {
        let upscaling_pipeline_id =
            upscaling_pipelines.specialize(&cache, &upscaling_pipeline, PulseUpscalingPipelineKey);
        commands
            .entity(view_entity)
            .insert(PulseUpscalingPipelineId(upscaling_pipeline_id));
    }
}
