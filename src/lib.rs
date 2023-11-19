use bevy::{
    core_pipeline::{core_2d, upscaling::UpscalingNode},
    prelude::*,
    render::{
        camera::CameraRenderGraph,
        render_graph::{RenderGraphApp, ViewNodeRunner},
        RenderApp,
    },
};

pub mod path_tracer;
use path_tracer::node::PulsePathTracerNode;

pub const PULSE_GRAPH: &str = "pulse_graph";

pub struct PulsePlugin;

impl Plugin for PulsePlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        let render_graph = app.sub_app_mut(RenderApp);

        render_graph
            .add_render_sub_graph(PULSE_GRAPH)
            .add_render_graph_node::<ViewNodeRunner<PulsePathTracerNode>>(
                PULSE_GRAPH,
                PulsePathTracerNode::NAME,
            );
        // .add_render_graph_node::<ViewNodeRunner<UpscalingNode>>(PULSE_GRAPH, "upscaling");

        // render_graph
        //     .add_render_graph_node::<ViewNodeRunner<PulsePathTracerNode>>(
        //         core_2d::graph::NAME,
        //         PulsePathTracerNode::NAME,
        //     )
        //     .add_render_graph_edges(
        //         core_2d::graph::NAME,
        //         &[
        //             core_2d::graph::node::MAIN_PASS,
        //             PulsePathTracerNode::NAME,
        //             core_2d::graph::node::BLOOM,
        //         ],
        //     );
    }
}
