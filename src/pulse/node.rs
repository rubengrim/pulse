use std::sync::atomic::Ordering;

use crate::{
    path_tracer::pipeline::{PulsePathTracerPipeline, PulsePathTracerPipelineId},
    scene::{PulseCanRender, PulseSceneBindGroup},
    utilities::*,
    PulsePathTracerAccumulationRenderTarget, PulseRenderTarget,
};
use bevy::{
    core_pipeline::prepass::ViewPrepassTextures,
    ecs::query::QueryItem,
    pbr::{
        deferred::{DeferredLightingPipeline, PbrDeferredLightingDepthId},
        MeshViewBindGroup, ViewFogUniformOffset, ViewLightsUniformOffset,
    },
    prelude::*,
    render::{
        render_graph::{InputSlotError, NodeRunError, RenderGraphContext, SlotLabel, ViewNode},
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::{ViewTarget, ViewUniformOffset, ViewUniforms},
    },
};

use super::{
    pipeline::{PulseGILayout, PulseGIPipeline},
    PulseGI,
};

pub struct PulseGINode;

impl PulseGINode {
    pub const NAME: &'static str = "pulse_gi_node";
}

impl ViewNode for PulseGINode {
    type ViewQuery = (
        &'static PulseGI,
        &'static PulseRenderTarget,
        &'static PulseGIPipeline,
        &'static ViewUniformOffset,
        &'static ViewLightsUniformOffset,
        &'static ViewFogUniformOffset,
        &'static MeshViewBindGroup,
    );

    fn update(&mut self, _world: &mut World) {}

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            pulse_gi,
            render_target,
            pulse_pipeline,
            view_offset,
            view_lights_offset,
            view_fog_offset,
            mesh_view_bind_group,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pulse_layout = world.resource::<PulseGILayout>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(pulse_pipeline.id) else {
            return Ok(());
        };

        let Some(scene_bind_group) = world.resource::<PulseSceneBindGroup>().0.clone() else {
            return Ok(());
        };

        let view_bind_group = render_context.render_device().create_bind_group(
            Some("pulse_gi_view_bind_group"),
            &pulse_layout.view_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(render_target.view()),
            }],
        );

        let mut compute_pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("pulse_gi_pass"),
                });

        compute_pass.set_bind_group(
            0,
            &mesh_view_bind_group.value,
            &[
                view_offset.offset,
                view_lights_offset.offset,
                view_fog_offset.offset,
            ],
        );
        compute_pass.set_bind_group(1, &scene_bind_group, &[]);
        compute_pass.set_bind_group(2, &view_bind_group, &[]);
        compute_pass.set_pipeline(&pipeline);
        let num_workgroups_x = (render_target.width() as f32 / 16.0).ceil() as u32;
        let num_workgroups_y = (render_target.height() as f32 / 16.0).ceil() as u32;
        compute_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);

        Ok(())
    }
}

impl FromWorld for PulseGINode {
    fn from_world(_world: &mut World) -> Self {
        Self
    }
}
