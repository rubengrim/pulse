use crate::{path_tracer::PulsePathTracerRenderTarget, utilities::create_uniform_buffer};
use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
        render_resource::*,
        renderer::{RenderContext, RenderQueue},
        view::ViewTarget,
    },
};

use crate::path_tracer::pt_upscaling::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct PulsePathTracerUpscalingNodeLabel;

pub struct PulsePathTracerUpscalingNode;

impl ViewNode for PulsePathTracerUpscalingNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static PulsePathTracerRenderTarget,
        &'static PulsePathTracerUpscalingPipelineId,
    );

    fn update(&mut self, _world: &mut World) {}

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, render_target, pipeline_id): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
            return Ok(());
        };

        let render_queue = world.resource::<RenderQueue>();

        let uniform = PulsePathTracerUpscalingUniform {
            width: render_target.width,
            height: render_target.height,
        };

        let uniform_buffer = create_uniform_buffer(
            uniform,
            Some("pulse_pt_upscaling_uniform"),
            render_context.render_device(),
            render_queue,
        );

        let layout = world.resource::<PulsePathTracerUpscalingLayout>();
        let bind_group = render_context.render_device().create_bind_group(
            Some("pulse_pt_upscaling_bind_group"),
            &layout.render_target_layout,
            &[
                // Uniform
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.into_binding(),
                },
                // Render target view
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&render_target.texture.default_view),
                },
                // Sampler
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(
                        &render_context
                            .render_device()
                            .create_sampler(&SamplerDescriptor::default()),
                    ),
                },
            ],
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("pulse_pt_upscaling_render_pass"),
            color_attachments: &[Some(view_target.get_color_attachment())],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Draw output texture to fullscreen triangle
        render_pass.set_render_pipeline(&pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

impl FromWorld for PulsePathTracerUpscalingNode {
    fn from_world(_world: &mut World) -> Self {
        Self
    }
}

#[derive(ShaderType, Clone, Copy)]
pub struct PulsePathTracerUpscalingUniform {
    width: u32,
    height: u32,
}
