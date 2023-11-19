use crate::PULSE_GRAPH;
use bevy::{
    asset::load_internal_asset,
    prelude::*,
    render::{
        camera::{CameraRenderGraph, ExtractedCamera},
        extract_component::ExtractComponent,
        render_resource::{SpecializedComputePipelines, SpecializedRenderPipelines},
        view::{ViewTarget, ViewUniformOffset},
        Render, RenderApp, RenderSet,
    },
    utils::Uuid,
};
use resources::PulsePathTracerPipeline;

use resources::*;

pub struct PulsePathTracerPlugin;

pub mod node;
pub mod resources;

pub const FULLSCREEN_TRIANGLE_VERTICES: &[f32] = &[0.0, 0.0, 10.0, 0.0, 10.0, 10.0];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

pub const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    },
];

const INDICES: &[u16] = &[0, 1, 2];

pub const PULSE_PATH_TRACER_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(187737725855836603431472235363477954946);

impl Plugin for PulsePathTracerPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            PULSE_PATH_TRACER_SHADER_HANDLE,
            "path_tracer.wgsl",
            Shader::from_wgsl
        );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<PulsePathTracerPipeline>()
            .init_resource::<SpecializedComputePipelines<PulsePathTracerPipeline>>()
            .init_resource::<SpecializedRenderPipelines<PulsePathTracerPipeline>>()
            .add_systems(
                Render,
                (prepare_pipelines, prepare_output_textures).in_set(RenderSet::Prepare),
            );
    }
}

#[derive(Component, Default, Clone, ExtractComponent)]
pub struct PulsePathTracer {
    test_value: f32,
}

#[derive(Bundle)]
pub struct PulsePathTracerCameraBundle {
    pub path_tracer: PulsePathTracer,
    pub camera: Camera,
    pub camera_render_graph: CameraRenderGraph,
    pub projection: Projection,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for PulsePathTracerCameraBundle {
    fn default() -> Self {
        Self {
            path_tracer: PulsePathTracer { test_value: 10.0 },
            camera: Default::default(),
            camera_render_graph: CameraRenderGraph::new(PULSE_GRAPH),
            projection: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}
