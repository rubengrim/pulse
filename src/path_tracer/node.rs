use crate::{
    create_render_target_bind_group,
    path_tracer::pipeline::{PulsePathTracerPipeline, PulsePathTracerPipelineId},
    PulseRenderTarget,
};
use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_resource::*,
        renderer::{RenderContext, RenderQueue},
        view::{ViewTarget, ViewUniformOffset, ViewUniforms},
    },
};

pub struct PulsePathTracerNode;

impl PulsePathTracerNode {
    pub const NAME: &'static str = "pulse_path_tracer_node";
}

impl ViewNode for PulsePathTracerNode {
    type ViewQuery = (
        &'static PulseRenderTarget,
        &'static PulsePathTracerPipelineId,
        &'static ViewUniformOffset,
    );

    fn update(&mut self, _world: &mut World) {}

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (render_target, pipeline_id, view_offset): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pulse_pipeline = world.resource::<PulsePathTracerPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(pipeline_id.0) else {
            return Ok(());
        };

        let view_bind_group = render_context.render_device().create_bind_group(
            Some("pulse_path_tracer_view_bind_group"),
            &pulse_pipeline.view_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: world.resource::<ViewUniforms>().uniforms.into_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(render_target.view()),
                },
            ],
        );

        let encoder = render_context.command_encoder();
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("pulse_path_tracer_pass"),
        });

        compute_pass.set_bind_group(0, &view_bind_group, &[view_offset.offset]);
        compute_pass.set_pipeline(&pipeline);
        let num_workgroups_x = (render_target.width() as f32 / 8.0).ceil() as u32;
        let num_workgroups_y = (render_target.height() as f32 / 8.0).ceil() as u32;
        compute_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);

        Ok(())
    }
}

impl FromWorld for PulsePathTracerNode {
    fn from_world(_world: &mut World) -> Self {
        Self
    }
}
