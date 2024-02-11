use crate::{
    pulse::{PulseGIRenderTarget, PulseShadowRenderTarget},
    upscaling::pipeline::PulseUpscalingLayout,
    utilities::create_uniform_buffer,
};
use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_resource::*,
        renderer::{RenderContext, RenderQueue},
        view::ViewTarget,
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
        &'static PulseGIRenderTarget,
        &'static PulseShadowRenderTarget,
        &'static PulseUpscalingPipelineId,
    );

    fn update(&mut self, _world: &mut World) {}

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, gi_target, shadow_target, pipeline_id): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
            return Ok(());
        };

        let render_queue = world.resource::<RenderQueue>();

        let uniform = PulseUpscalingUniform {
            width: gi_target.width,
            height: gi_target.height,
        };

        let uniform_buffer = create_uniform_buffer(
            uniform,
            Some("pulse_upscaling_uniform"),
            render_context.render_device(),
            render_queue,
        );

        let layout = world.resource::<PulseUpscalingLayout>();
        let bind_group = render_context.render_device().create_bind_group(
            Some("pulse_upscaling_bind_group"),
            &layout.render_target_layout,
            &[
                // Uniform
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.into_binding(),
                },
                // GI target view
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&gi_target.texture.default_view),
                },
                // Shadow target view
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&shadow_target.texture.default_view),
                },
                // Sampler
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(
                        &render_context
                            .render_device()
                            .create_sampler(&SamplerDescriptor::default()),
                    ),
                },
            ],
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

#[derive(ShaderType, Clone, Copy)]
pub struct PulseUpscalingUniform {
    width: u32,
    height: u32,
}
