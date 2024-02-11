use std::sync::atomic::Ordering;

use crate::{
    path_tracer::pipeline::{PulsePathTracerPipeline, PulsePathTracerPipelineId},
    scene::{PulseCanRender, PulseSceneBindGroup},
    utilities::*,
    PulsePathTracerAccumulationRenderTarget, PulseRenderTarget,
};
use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::{ViewUniformOffset, ViewUniforms},
    },
};

use super::{PulsePathTracer, PulsePathTracerUniform};

pub struct PulsePathTracerNode;

impl PulsePathTracerNode {
    pub const NAME: &'static str = "pulse_path_tracer_node";
}

impl ViewNode for PulsePathTracerNode {
    type ViewQuery = (
        &'static PulsePathTracer,
        &'static PulseRenderTarget,
        &'static PulsePathTracerAccumulationRenderTarget, // Hack for now
        &'static PulsePathTracerPipelineId,
        &'static ViewUniformOffset,
    );

    fn update(&mut self, _world: &mut World) {}

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (pulse_path_tracer, render_target, acc_texture, pipeline_id, view_offset): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !world.resource::<PulseCanRender>().0 {
            return Ok(());
        }

        let pulse_pipeline = world.resource::<PulsePathTracerPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(pipeline_id.0) else {
            return Ok(());
        };

        let Some(scene_bind_group) = world.resource::<PulseSceneBindGroup>().0.clone() else {
            return Ok(());
        };

        let device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();
        let path_tracer_uniform = create_uniform_buffer(
            PulsePathTracerUniform {
                previous_sample_count: pulse_path_tracer
                    .sample_count
                    .fetch_add(1, Ordering::SeqCst),
            },
            Some("pulse_path_tracer_uniform_buffer"),
            device,
            queue,
        );

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
                    resource: path_tracer_uniform.into_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(render_target.view()),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&acc_texture.0.default_view),
                },
            ],
        );

        let mut compute_pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("pulse_path_tracer_pass"),
                });

        compute_pass.set_bind_group(0, &scene_bind_group, &[]);
        compute_pass.set_bind_group(1, &view_bind_group, &[view_offset.offset]);
        compute_pass.set_pipeline(&pipeline);
        let num_workgroups_x = (render_target.width() as f32 / 16.0).ceil() as u32;
        let num_workgroups_y = (render_target.height() as f32 / 16.0).ceil() as u32;
        compute_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);

        Ok(())
    }
}

impl FromWorld for PulsePathTracerNode {
    fn from_world(_world: &mut World) -> Self {
        Self
    }
}
