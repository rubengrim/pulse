use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use crate::{
    path_tracer::PulsePathTracerNode, scene::PulseSceneBindGroupLayout,
    upscaling::PulseUpscalingNode, PulsePathTracerAccumulationRenderTarget, PulseRenderTarget,
    PULSE_PATH_TRACER_GRAPH,
};
use bevy::{
    asset::load_internal_asset,
    core_pipeline::core_3d,
    prelude::*,
    render::{
        camera::{CameraRenderGraph, ExtractedCamera},
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_graph::{RenderGraphApp, ViewNodeRunner},
        render_resource::*,
        renderer::RenderDevice,
        texture::TextureCache,
        Render, RenderApp, RenderSet,
    },
    transform::TransformSystem,
};

pub mod node;
pub mod pipeline;

use node::*;
use pipeline::*;

pub const PULSE_GI_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(114283296390698627303979451153780021723);

pub struct PulseRealtimePlugin;

impl Plugin for PulseRealtimePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, PULSE_GI_SHADER_HANDLE, "gi.wgsl", Shader::from_wgsl);

        app.add_plugins(ExtractComponentPlugin::<PulseGI>::default());
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<PulseGILayout>()
            .init_resource::<SpecializedComputePipelines<PulseGILayout>>()
            .add_systems(
                Render,
                (prepare_gi_render_targets, prepare_gi_pipelines).in_set(RenderSet::Prepare),
            );

        render_app
            .add_render_graph_node::<ViewNodeRunner<PulseGINode>>(
                core_3d::graph::NAME,
                PulseGINode::NAME,
            )
            .add_render_graph_node::<ViewNodeRunner<PulseUpscalingNode>>(
                core_3d::graph::NAME,
                PulseUpscalingNode::NAME,
            )
            .add_render_graph_edges(
                core_3d::graph::NAME,
                &[
                    core_3d::graph::node::END_MAIN_PASS,
                    PulseGINode::NAME,
                    PulseUpscalingNode::NAME,
                    core_3d::graph::node::TONEMAPPING,
                ],
            );
    }
}

#[derive(Component, ExtractComponent, Clone)]
pub struct PulseGI;

// Detta är efterblivet views måste kunna ha flera render targets.
pub fn prepare_gi_render_targets(
    views: Query<(Entity, &ExtractedCamera)>,
    device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    mut commands: Commands,
) {
    for (entity, camera) in &views {
        if let Some(target_size) = camera.physical_target_size {
            let render_target = PulseRenderTarget::new(
                target_size.x,
                target_size.y,
                None,
                &mut texture_cache,
                &device,
            );

            commands.entity(entity).insert(render_target);
        }
    }
}
