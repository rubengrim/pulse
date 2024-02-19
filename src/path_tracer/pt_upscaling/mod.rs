use bevy::{
    asset::load_internal_asset,
    prelude::*,
    render::{render_resource::*, Render, RenderApp, RenderSet},
};

pub mod node;
pub mod pipeline;

pub use node::*;
pub use pipeline::*;

pub const PULSE_PT_UPSCALING_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(72778439306338317084755245320098158356);

pub struct PulsePathTracerUpscalingPlugin;

impl Plugin for PulsePathTracerUpscalingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            PULSE_PT_UPSCALING_SHADER_HANDLE,
            "pt_upscaling.wgsl",
            Shader::from_wgsl
        );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<PulsePathTracerUpscalingLayout>()
            .init_resource::<SpecializedRenderPipelines<PulsePathTracerUpscalingLayout>>();

        render_app.add_systems(
            Render,
            prepare_pt_upscaling_pipelines.in_set(RenderSet::Prepare),
        );
    }
}
