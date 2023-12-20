use crate::{PulseRenderTarget, PULSE_GRAPH};
use bevy::{
    asset::load_internal_asset,
    prelude::*,
    render::{
        camera::{CameraRenderGraph, ExtractedCamera},
        extract_component::ExtractComponent,
        render_resource::*,
        renderer::RenderDevice,
        texture::TextureCache,
        Render, RenderApp, RenderSet,
    },
};

pub mod node;
pub use node::*;

pub mod pipeline;
pub use pipeline::*;

pub const PULSE_PATH_TRACER_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(187737725855836603431472235363477954946);

pub struct PulsePathTracerPlugin;

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
            .add_systems(
                Render,
                (prepare_pipelines, prepare_render_targets)
                    .in_set(RenderSet::Prepare)
                    .before(crate::upscaling::prepare_upscaling_pipelines),
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
            camera: Camera {
                hdr: true,
                ..default()
            },
            camera_render_graph: CameraRenderGraph::new(PULSE_GRAPH),
            projection: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

pub fn prepare_render_targets(
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
