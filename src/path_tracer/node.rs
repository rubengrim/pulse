use std::sync::atomic::Ordering;

use crate::{
    path_tracer::pipeline::PulsePathTracerPipeline, scene::PulseSceneBindGroup, utilities::*,
};
use bevy::{
    core_pipeline::prepass::ViewPrepassTextures,
    ecs::query::QueryItem,
    pbr::{MeshViewBindGroup, ViewFogUniformOffset, ViewLightsUniformOffset},
    prelude::*,
    render::{
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::{ViewTarget, ViewUniformOffset, ViewUniforms},
    },
};

use super::{
    PulsePathTracerCamera, PulsePathTracerLayout, PulsePathTracerRenderTarget,
    PulsePathTracerUniform,
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct PulsePathTracerNodeLabel;

pub struct PulsePathTracerNode;

impl ViewNode for PulsePathTracerNode {
    type ViewQuery = (
        &'static PulsePathTracerCamera,
        &'static PulsePathTracerRenderTarget,
        &'static PulsePathTracerPipeline,
        &'static ViewUniformOffset,
        &'static ViewPrepassTextures,
    );

    fn update(&mut self, _world: &mut World) {}

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (path_tracer, render_target, pipeline, view_offset, prepass_textures): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let layout = world.resource::<PulsePathTracerLayout>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(pipeline.id) else {
            return Ok(());
        };

        let Some(scene_bind_group) = world.resource::<PulseSceneBindGroup>().0.clone() else {
            return Ok(());
        };

        let acc_count = path_tracer
            .accumulation_count
            .fetch_add(1, Ordering::SeqCst);

        let device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();
        let path_tracer_uniform = create_uniform_buffer(
            PulsePathTracerUniform {
                width: render_target.width,
                height: render_target.height,
                accumulation_count: acc_count,
            },
            Some("pulse_path_tracer_uniform_buffer"),
            device,
            queue,
        );

        let view_uniforms = world.resource::<ViewUniforms>();
        let view_bind_group = render_context.render_device().create_bind_group(
            Some("pulse_path_tracer_view_bind_group"),
            &layout.view_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: view_uniforms.uniforms.binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &prepass_textures.deferred_view().unwrap(),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&prepass_textures.depth_view().unwrap()),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&render_target.texture.default_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: path_tracer_uniform.into_binding(),
                },
            ],
        );

        let mut compute_pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("pulse_path_tracer_pass"),
                    timestamp_writes: None,
                });

        compute_pass.set_bind_group(0, &scene_bind_group, &[]);
        compute_pass.set_bind_group(1, &view_bind_group, &[view_offset.offset]);
        compute_pass.set_pipeline(&pipeline);
        let num_workgroups_x = (render_target.width as f32 / 16.0).ceil() as u32;
        let num_workgroups_y = (render_target.height as f32 / 16.0).ceil() as u32;
        compute_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);

        Ok(())
    }
}

impl FromWorld for PulsePathTracerNode {
    fn from_world(_world: &mut World) -> Self {
        Self
    }
}
