use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use bevy::{
    asset::load_internal_asset,
    core_pipeline::core_3d,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_graph::{RenderGraphApp, ViewNodeRunner},
        render_resource::*,
        renderer::RenderDevice,
        texture::{CachedTexture, TextureCache},
        Render, RenderApp, RenderSet,
    },
    transform::TransformSystem,
};

pub mod node;
pub mod pipeline;
pub mod pt_upscaling;

pub use node::*;
pub use pipeline::*;
pub use pt_upscaling::*;

pub const PULSE_PATH_TRACER_GRAPH: &str = "pulse_path_tracer_graph";

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

        app.add_plugins((
            ExtractComponentPlugin::<PulsePathTracerCamera>::default(),
            PulsePathTracerUpscalingPlugin,
        ));

        app.add_systems(
            PostUpdate,
            reset_accumulation_on_movement.after(TransformSystem::TransformPropagate),
        );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .add_render_graph_node::<ViewNodeRunner<PulsePathTracerNode>>(
                core_3d::graph::Core3d,
                PulsePathTracerNodeLabel,
            )
            .add_render_graph_node::<ViewNodeRunner<PulsePathTracerUpscalingNode>>(
                core_3d::graph::Core3d,
                PulsePathTracerUpscalingNodeLabel,
            )
            .add_render_graph_edges(
                core_3d::graph::Core3d,
                (
                    core_3d::graph::Node3d::EndMainPass,
                    PulsePathTracerNodeLabel,
                    PulsePathTracerUpscalingNodeLabel,
                    core_3d::graph::Node3d::Bloom,
                ),
            );

        render_app
            .init_resource::<PulsePathTracerLayout>()
            .init_resource::<SpecializedComputePipelines<PulsePathTracerLayout>>()
            .add_systems(
                Render,
                (
                    prepare_path_tracer_pipelines,
                    prepare_path_tracer_render_targets,
                )
                    .in_set(RenderSet::Prepare),
            );
    }
}

#[derive(ShaderType)]
pub struct PulsePathTracerUniform {
    pub width: u32,
    pub height: u32,
    pub accumulation_count: u32,
}

#[derive(Component, Default, Clone, ExtractComponent)]
pub struct PulsePathTracerCamera {
    pub resolution: Option<UVec2>,
    pub accumulation_count: u32,
    pub previous_transform: GlobalTransform,
}

fn reset_accumulation_on_movement(
    mut views: Query<(&GlobalTransform, &mut PulsePathTracerCamera)>,
) {
    for (current_transform, mut path_tracer) in views.iter_mut() {
        if *current_transform != path_tracer.previous_transform {
            path_tracer.accumulation_count = 0;
        } else {
            path_tracer.accumulation_count += 1;
        }
        path_tracer.previous_transform = *current_transform;
    }
}

#[derive(Component)]
pub struct PulsePathTracerRenderTarget {
    pub texture: CachedTexture,
    pub width: u32,
    pub height: u32,
}

fn prepare_path_tracer_render_targets(
    views: Query<(Entity, &ExtractedCamera, &PulsePathTracerCamera)>,
    device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    mut commands: Commands,
) {
    let mut get_texture = |width: u32, height: u32, label: Option<&'static str>| {
        texture_cache.get(
            &device,
            TextureDescriptor {
                label,
                size: Extent3d {
                    width,
                    height,
                    ..default()
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba32Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[TextureFormat::Rgba32Float],
            },
        )
    };

    for (entity, camera, path_tracer) in &views {
        let res = path_tracer
            .resolution
            .unwrap_or_else(|| camera.physical_target_size.unwrap_or(UVec2::new(720, 480)));

        let target = PulsePathTracerRenderTarget {
            texture: get_texture(res.x, res.y, Some("pulse_path_tracer_target_texture")),
            width: res.x,
            height: res.y,
        };

        commands.entity(entity).insert(target);
    }
}
