use bevy::{
    asset::load_internal_asset,
    prelude::*,
    render::{render_resource::*, Render, RenderApp, RenderSet},
};

pub mod node;
pub use node::*;

pub mod pipeline;
pub use pipeline::*;

pub const PULSE_UPSCALING_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(338718014948591213280805766994766947065);

pub struct PulseUpscalingPlugin;

impl Plugin for PulseUpscalingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            PULSE_UPSCALING_SHADER_HANDLE,
            "upscaling.wgsl",
            Shader::from_wgsl
        );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<PulseUpscalingLayout>()
            .init_resource::<SpecializedRenderPipelines<PulseUpscalingLayout>>();

        render_app.add_systems(
            Render,
            prepare_upscaling_pipelines.in_set(RenderSet::Prepare),
        );
    }
}
