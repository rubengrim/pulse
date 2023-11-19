use super::{PulsePathTracer, PULSE_PATH_TRACER_SHADER_HANDLE};
use bevy::{
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_resource::*,
        renderer::RenderDevice,
        texture::{CachedTexture, TextureCache},
        view::{ViewTarget, ViewUniform, ViewUniforms},
    },
};
use std::mem::size_of;

#[derive(Resource)]
pub struct PulsePathTracerPipeline {
    pub view_layout: BindGroupLayout,
}

impl FromWorld for PulsePathTracerPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let view_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("pulse_view_bind_group_layout"),
            entries: &[
                // View uniforms
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX,
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
                    visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: ViewTarget::TEXTURE_FORMAT_HDR,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Output texture sampler
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        Self { view_layout }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PulsePathTracerComputePipelineKey;

impl SpecializedComputePipeline for PulsePathTracerPipeline {
    type Key = PulsePathTracerComputePipelineKey;

    fn specialize(&self, _key: Self::Key) -> ComputePipelineDescriptor {
        ComputePipelineDescriptor {
            label: Some("pulse_path_tracer_pipeline".into()),
            layout: vec![self.view_layout.clone()],
            push_constant_ranges: vec![],
            shader: PULSE_PATH_TRACER_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "path_trace".into(),
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PulsePathTracerRenderPipelineKey;

impl SpecializedRenderPipeline for PulsePathTracerPipeline {
    type Key = PulsePathTracerRenderPipelineKey;

    fn specialize(&self, _key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("pulse_path_tracer_render_pipeline".into()),
            layout: vec![self.view_layout.clone()],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: PULSE_PATH_TRACER_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "fullscreen_vertex".into(),
                buffers: vec![VertexBufferLayout {
                    array_stride: (size_of::<[f32; 6]>()) as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![
                        VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float32x3,
                        },
                        VertexAttribute {
                            offset: std::mem::size_of::<[f32; 3]>() as u64,
                            shader_location: 1,
                            format: VertexFormat::Float32x3,
                        },
                    ],
                }],
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
                shader: PULSE_PATH_TRACER_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "fullscreen_fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Bgra8UnormSrgb,
                    // format: TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
        }
    }
}

#[derive(Component)]
pub struct PulsePathTracerComputePipelineIds {
    pub compute_pipeline_id: CachedComputePipelineId,
    pub render_pipeline_id: CachedRenderPipelineId,
}

pub fn prepare_pipelines(
    views: Query<Entity, With<ExtractedCamera>>,
    mut commands: Commands,
    mut compute_pipelines: ResMut<SpecializedComputePipelines<PulsePathTracerPipeline>>,
    mut render_pipelines: ResMut<SpecializedRenderPipelines<PulsePathTracerPipeline>>,
    cache: Res<PipelineCache>,
    pipeline: Res<PulsePathTracerPipeline>,
) {
    for view_entity in &views {
        let compute_pipeline_id =
            compute_pipelines.specialize(&cache, &pipeline, PulsePathTracerComputePipelineKey);
        let render_pipeline_id =
            render_pipelines.specialize(&cache, &pipeline, PulsePathTracerRenderPipelineKey);
        commands
            .entity(view_entity)
            .insert(PulsePathTracerComputePipelineIds {
                compute_pipeline_id,
                render_pipeline_id,
            });
    }
}

#[derive(Component)]
pub struct PulsePathTracerOutputTexture {
    pub texture: CachedTexture,
    pub sampler: Sampler,
}

pub fn prepare_output_textures(
    views: Query<(Entity, &ExtractedCamera)>,
    device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    mut commands: Commands,
) {
    for (entity, camera) in &views {
        if let Some(target_size) = camera.physical_target_size {
            let texture = texture_cache.get(
                &device,
                TextureDescriptor {
                    label: Some("path_tracer_output_texture"),
                    size: Extent3d {
                        width: target_size.x,
                        height: target_size.y,
                        ..default()
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: ViewTarget::TEXTURE_FORMAT_HDR,
                    usage: TextureUsages::STORAGE_BINDING,
                    view_formats: &[ViewTarget::TEXTURE_FORMAT_HDR],
                },
            );

            let sampler = device.create_sampler(&SamplerDescriptor::default());

            commands
                .entity(entity)
                .insert(PulsePathTracerOutputTexture { texture, sampler });
        }
    }
}

pub fn create_view_bind_group(
    view_uniforms: &ViewUniforms,
    layout: &BindGroupLayout,
    device: &RenderDevice,
    output_texture: &PulsePathTracerOutputTexture,
) -> Option<BindGroup> {
    view_uniforms.uniforms.binding().map(|view_uniforms| {
        device.create_bind_group(
            Some("pulse_view_bind_group"),
            layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: view_uniforms.clone(),
                },
                // Output texture view
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&output_texture.texture.default_view),
                },
                // Output texture sampler
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&output_texture.sampler),
                },
            ],
        )
    })
}
