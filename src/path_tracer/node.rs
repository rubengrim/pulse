use super::{
    resources::{
        create_view_bind_group, PulsePathTracerComputePipelineIds, PulsePathTracerOutputTexture,
        PulsePathTracerPipeline,
    },
    PulsePathTracer,
};
use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_resource::{
            BufferInitDescriptor, BufferUsages, ComputePassDescriptor, IndexFormat, LoadOp,
            Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
        },
        renderer::RenderContext,
        texture::TextureCache,
        view::{ViewTarget, ViewUniformOffset, ViewUniforms},
    },
};

pub struct PulsePathTracerNode;

impl PulsePathTracerNode {
    pub const NAME: &'static str = "pulse_path_tracer_node";
}

impl ViewNode for PulsePathTracerNode {
    type ViewQuery = (
        // &'static PulsePathTracer,
        &'static ViewTarget,
        &'static ViewUniformOffset,
        &'static PulsePathTracerComputePipelineIds,
        &'static PulsePathTracerOutputTexture,
        &'static ExtractedCamera,
    );

    fn update(&mut self, _world: &mut World) {
        // info!("Updating node");
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, view_uniform_offset, pipeline_ids, output_texture, camera): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        // let device = render_context.render_device();
        let view_uniforms = world.resource::<ViewUniforms>();
        let pulse_pipeline = world.resource::<PulsePathTracerPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        // info!("Running node");

        let (
            Some(compute_pipeline),
            Some(render_pipeline),
            Some(view_bind_group),
            Some(physical_target_size),
        ) = (
            pipeline_cache.get_compute_pipeline(pipeline_ids.compute_pipeline_id),
            pipeline_cache.get_render_pipeline(pipeline_ids.render_pipeline_id),
            create_view_bind_group(
                view_uniforms,
                &pulse_pipeline.view_layout,
                render_context.render_device(),
                output_texture,
            ),
            camera.physical_target_size,
        )
        else {
            return Ok(());
        };

        {
            let encoder = render_context.command_encoder();
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("pulse_path_tracer_compute_pass"),
            });

            compute_pass.set_bind_group(0, &view_bind_group, &[view_uniform_offset.offset]);
            compute_pass.set_pipeline(&compute_pipeline);
            let num_workgroups_x = (physical_target_size.x as f32 / 8.0).ceil() as u32;
            let num_workgroups_y = (physical_target_size.y as f32 / 8.0).ceil() as u32;
            compute_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);
        }

        let vertex_buffer =
            render_context
                .render_device()
                .create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("pulse_upscaling_vertex_buffer"),
                    contents: bytemuck::cast_slice(super::VERTICES),
                    usage: BufferUsages::VERTEX,
                });

        let index_buffer =
            render_context
                .render_device()
                .create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("pulse_upscaling_index_buffer"),
                    contents: bytemuck::cast_slice(super::INDICES),
                    usage: BufferUsages::INDEX,
                });

        {
            // let post_process = view_target.post_process_write();
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("pulse_path_tracer_render_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: view_target.out_texture(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::WHITE.into()),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            // let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            //     label: Some("pulse_path_tracer_render_pass"),
            //     color_attachments: &[Some(view_target.get_color_attachment(Operations {
            //         load: LoadOp::Clear(Color::GREEN.into()),
            //         store: true,
            //     }))],
            //     depth_stencil_attachment: None,
            // });

            // Draw output texture to fullscreen triangle
            render_pass.set_render_pipeline(&render_pipeline);
            render_pass.set_bind_group(0, &view_bind_group, &[view_uniform_offset.offset]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            // render_pass.set_index_buffer(index_buffer.slice(..), 0, IndexFormat::Uint16);
            // render_pass.draw_indexed(0..3, 0, 0..1);
            render_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}

impl FromWorld for PulsePathTracerNode {
    fn from_world(_world: &mut World) -> Self {
        Self
    }
}
