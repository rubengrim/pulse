use crate::{create_render_target_bind_group, PulseRenderTarget};
use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::{ViewTarget, ViewUniformOffset, ViewUniforms},
    },
};

use crate::upscaling::*;

pub struct PulseUpscalingNode;
impl PulseUpscalingNode {
    pub const NAME: &'static str = "pulse_upscaling";
}

impl ViewNode for PulseUpscalingNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static PulseRenderTarget,
        &'static PulseUpscalingPipelineId,
    );

    fn update(&mut self, _world: &mut World) {}

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, pulse_render_target, pipeline_id): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
            return Ok(());
        };

        let upscaling_pipeline = world.resource::<PulseUpscalingPipeline>();
        let render_queue = world.resource::<RenderQueue>();
        let bind_group = create_render_target_bind_group(
            pulse_render_target,
            &upscaling_pipeline.render_target_layout,
            &render_context.render_device(),
            &render_queue,
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("pulse_upscaling_render_pass"),
            color_attachments: &[Some(view_target.get_color_attachment(Operations {
                load: LoadOp::Load,
                store: true,
            }))],
            depth_stencil_attachment: None,
        });

        // Draw output texture to fullscreen triangle
        render_pass.set_render_pipeline(&pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

impl FromWorld for PulseUpscalingNode {
    fn from_world(_world: &mut World) -> Self {
        Self
    }
}
