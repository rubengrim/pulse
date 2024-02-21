use bevy::{prelude::*, render::RenderApp};

// pub mod diagnostics;
pub mod path_tracer;
pub mod pulse;
pub mod scene;
pub mod upscaling;
pub mod utilities;

// use diagnostics::*;
use scene::*;

pub struct PulsePlugin;

impl Plugin for PulsePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PulseScenePlugin);
    }

    fn finish(&self, app: &mut App) {
        let _render_app = app.sub_app_mut(RenderApp);

        // TODO: Implement this properly. Not sure if I use any non-standard features though
        // let required_features = WgpuFeatures::TIMESTAMP_QUERY;
        // match render_app.world.get_resource::<RenderDevice>() {
        //     Some(render_device) => {
        //         if !render_device.features().contains(required_features) {
        //             error!("All required wgpu features are not supported");
        //             return;
        //         }
        //     }
        //     _ => {
        //         warn!("RenderDevice not found");
        //     }
        // }
    }
}
