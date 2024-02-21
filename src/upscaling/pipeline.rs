use super::PULSE_UPSCALING_SHADER_HANDLE;
use bevy::{
    prelude::*,
    render::{camera::ExtractedCamera, render_resource::*, renderer::RenderDevice},
};

#[derive(Resource)]
pub struct PulseUpscalingLayout {
    pub render_target_layout: BindGroupLayout,
}

impl FromWorld for PulseUpscalingLayout {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        Self {
            render_target_layout: device.create_bind_group_layout(
                Some("pulse_view_bind_group_layout"),
                &[
                    // Render target uniforms
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // GI target view
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Shadow target view
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Sampler
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            ),
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PulseUpscalingPipelineKey;

impl SpecializedRenderPipeline for PulseUpscalingLayout {
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
                    format: TextureFormat::Rgba16Float,
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
    mut upscaling_pipelines: ResMut<SpecializedRenderPipelines<PulseUpscalingLayout>>,
    upscaling_pipeline: Res<PulseUpscalingLayout>,
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
