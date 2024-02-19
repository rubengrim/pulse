use crate::upscaling::{PulseUpscalingNode, PulseUpscalingPlugin};
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

        app.add_plugins((
            ExtractComponentPlugin::<PulseCamera>::default(),
            PulseUpscalingPlugin,
        ));
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<PulseGILayout>()
            .init_resource::<SpecializedComputePipelines<PulseGILayout>>()
            .add_systems(
                Render,
                (prepare_pulse_render_targets, prepare_gi_pipelines).in_set(RenderSet::Prepare),
            );

        render_app
            .add_render_graph_node::<ViewNodeRunner<PulseNode>>(
                core_3d::graph::NAME,
                PulseNode::NAME,
            )
            .add_render_graph_node::<ViewNodeRunner<PulseUpscalingNode>>(
                core_3d::graph::NAME,
                PulseUpscalingNode::NAME,
            )
            .add_render_graph_edges(
                core_3d::graph::NAME,
                &[
                    core_3d::graph::node::END_MAIN_PASS,
                    PulseNode::NAME,
                    PulseUpscalingNode::NAME,
                    core_3d::graph::node::TONEMAPPING,
                ],
            );
    }
}

#[derive(Component, ExtractComponent, Clone, Default)]
pub struct PulseCamera {
    // Will use camera target size if `None`
    pub resolution: Option<UVec2>,
}

#[derive(Component)]
pub struct PulseGIRenderTarget {
    pub texture: CachedTexture,
    pub width: u32,
    pub height: u32,
}

#[derive(Component)]
pub struct PulseShadowRenderTarget {
    pub texture: CachedTexture,
    pub width: u32,
    pub height: u32,
}

fn prepare_pulse_render_targets(
    views: Query<(Entity, &ExtractedCamera, &PulseCamera)>,
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

    for (entity, camera, pulse_settings) in &views {
        let res = pulse_settings
            .resolution
            .unwrap_or_else(|| camera.physical_target_size.unwrap_or(UVec2::new(720, 480)));

        let gi_target = PulseGIRenderTarget {
            texture: get_texture(res.x, res.y, Some("pulse_gi_target_texture")),
            width: res.x,
            height: res.y,
        };

        let shadow_target = PulseShadowRenderTarget {
            texture: get_texture(res.x, res.y, Some("pulse_shadow_target_texture")),
            width: res.x,
            height: res.y,
        };

        commands.entity(entity).insert((gi_target, shadow_target));
    }
}
