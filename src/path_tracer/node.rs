use crate::{
    create_render_target_bind_group,
    path_tracer::pipeline::{PulsePathTracerPipeline, PulsePathTracerPipelineId},
    scene::PulseSceneBindGroup,
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
use wgpu::{QuerySetDescriptor, QueryType};

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

        let Some(scene_bind_group) = world.resource::<PulseSceneBindGroup>().0.clone() else {
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

        let query_set = render_context
            .render_device()
            .wgpu_device()
            .create_query_set(&QuerySetDescriptor {
                label: None,
                ty: QueryType::Timestamp,
                count: 2,
            });

        {
            let mut compute_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("pulse_path_tracer_pass"),
                    });

            compute_pass.write_timestamp(&query_set, 0);
            compute_pass.push_debug_group("path_trace");

            compute_pass.set_bind_group(0, &scene_bind_group, &[]);
            compute_pass.set_bind_group(1, &view_bind_group, &[view_offset.offset]);
            compute_pass.set_pipeline(&pipeline);
            let num_workgroups_x = (render_target.width() as f32 / 16.0).ceil() as u32;
            let num_workgroups_y = (render_target.height() as f32 / 16.0).ceil() as u32;

            compute_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);

            compute_pass.pop_debug_group();
            compute_pass.write_timestamp(&query_set, 1);
        }

        let timestamp_buffer =
            render_context
                .render_device()
                .create_buffer_with_data(&BufferInitDescriptor {
                    label: None,
                    contents: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
                    usage: BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
                });

        // let timestamp_buffer = render_context
        //     .render_device()
        //     .create_buffer(&BufferDescriptor {
        //         label: None,
        //         size: 8 * 2,
        //         usage: BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
        //         mapped_at_creation: false,
        //     });

        // Read back timestamp data.
        let mut command_encoder = render_context
            .render_device()
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        command_encoder.resolve_query_set(&query_set, 0..2, &timestamp_buffer, 0);

        let readback_buffer = render_context
            .render_device()
            .create_buffer(&BufferDescriptor {
                label: None,
                size: timestamp_buffer.size(),
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

        command_encoder.copy_buffer_to_buffer(
            &timestamp_buffer,
            0,
            &readback_buffer,
            0,
            timestamp_buffer.size(),
        );

        // world
        //     .resource::<RenderQueue>()
        //     .submit([command_encoder.finish()]);

        // readback_buffer
        //     .clone()
        //     .slice(..)
        //     .map_async(wgpu::MapMode::Read, move |result| {
        //         let err = result.err();
        //         if err.is_some() {
        //             panic!("{}", err.unwrap().to_string());
        //         }
        //         let contents = readback_buffer.slice(..).get_mapped_range();
        //         let readback = contents
        //             .chunks_exact(std::mem::size_of::<u8>())
        //             .map(|bytes| u8::from_ne_bytes(bytes.try_into().unwrap()))
        //             .collect::<Vec<u8>>();
        //         println!("Output: {readback:?}");
        //     });

        Ok(())
    }
}

impl FromWorld for PulsePathTracerNode {
    fn from_world(_world: &mut World) -> Self {
        Self
    }
}
