use crate::{scene::PulseSceneBindGroup, utilities::create_uniform_buffer};
use bevy::{
    ecs::query::QueryItem,
    pbr::{MeshViewBindGroup, ViewFogUniformOffset, ViewLightsUniformOffset},
    prelude::*,
    render::{
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
        render_resource::*,
        renderer::{RenderContext, RenderQueue},
        view::ViewUniformOffset,
    },
};

use super::{
    pipeline::{PulseGILayout, PulseGIPipeline},
    PulseCamera, PulseGIRenderTarget, PulseShadowRenderTarget,
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct PulseNodeLabel;

pub struct PulseNode;

impl ViewNode for PulseNode {
    type ViewQuery = (
        &'static PulseCamera,
        &'static PulseGIRenderTarget,
        &'static PulseShadowRenderTarget,
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
            _pulse_settings,
            gi_render_target,
            shadow_render_target,
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

        let uniform = PulseUniform {
            width: gi_render_target.width,
            height: gi_render_target.height,
        };

        let uniform_buffer = create_uniform_buffer(
            uniform,
            Some("pulse_resolution_uniform"),
            render_context.render_device(),
            world.resource::<RenderQueue>(),
        );

        let view_bind_group = render_context.render_device().create_bind_group(
            Some("pulse_view_bind_group"),
            &pulse_layout.view_layout,
            &[
                // GI target view
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&gi_render_target.texture.default_view),
                },
                // Shadow target view
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &shadow_render_target.texture.default_view,
                    ),
                },
                // Resolution uniform
                BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.binding().unwrap(),
                },
            ],
        );

        let mut compute_pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("pulse_pass"),
                    timestamp_writes: None,
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
        let num_workgroups_x = (gi_render_target.width as f32 / 16.0).ceil() as u32;
        let num_workgroups_y = (gi_render_target.height as f32 / 16.0).ceil() as u32;
        compute_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);

        Ok(())
    }
}

impl FromWorld for PulseNode {
    fn from_world(_world: &mut World) -> Self {
        Self
    }
}

#[derive(ShaderType, Clone, Copy)]
pub struct PulseUniform {
    width: u32,
    height: u32,
    // sample_count: u32,
}
