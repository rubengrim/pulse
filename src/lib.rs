use bevy::{
    prelude::*,
    render::{
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::{CachedTexture, TextureCache},
        RenderApp,
    },
};

pub mod diagnostics;
pub mod path_tracer;
pub mod pulse;
pub mod scene;
pub mod upscaling;
pub mod utilities;

use diagnostics::*;
use path_tracer::*;
use scene::*;
use upscaling::*;
use utilities::*;

pub struct PulsePlugin;

impl Plugin for PulsePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PulseDiagnosticsPlugin,
            PulseScenePlugin,
            PulseUpscalingPlugin,
        ));
    }

    fn finish(&self, app: &mut App) {
        let _render_app = app.sub_app_mut(RenderApp);

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

#[derive(Component)]
pub struct PulsePathTracerAccumulationRenderTarget(pub CachedTexture);

#[derive(Component)]
pub struct PulseRenderTarget {
    texture: CachedTexture,
    width: u32,
    height: u32,
}

impl PulseRenderTarget {
    pub const TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba32Float;

    fn view(&self) -> &TextureView {
        &self.texture.default_view
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn new(
        width: u32,
        height: u32,
        label: Option<&'static str>,
        texture_cache: &mut TextureCache,
        render_device: &RenderDevice,
    ) -> Self {
        let label = match label {
            Some(val) => Some(val),
            None => Some("pulse_render_target"),
        };

        let texture = texture_cache.get(
            render_device,
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
                format: PulseRenderTarget::TEXTURE_FORMAT,
                usage: TextureUsages::STORAGE_BINDING,
                view_formats: &[PulseRenderTarget::TEXTURE_FORMAT],
            },
        );

        Self {
            texture,
            width,
            height,
        }
    }
}

#[derive(ShaderType, Clone, Copy)]
pub struct PulseRenderTargetUniform {
    width: u32,
    height: u32,
}

pub fn create_render_target_layout(render_device: &RenderDevice) -> BindGroupLayout {
    render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("pulse_view_bind_group_layout"),
        entries: &[
            // Render target uniforms
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Render target view
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::StorageTexture {
                    access: StorageTextureAccess::ReadWrite,
                    format: PulseRenderTarget::TEXTURE_FORMAT,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            },
        ],
    })
}

pub fn create_render_target_bind_group(
    render_target: &PulseRenderTarget,
    layout: &BindGroupLayout,
    render_device: &RenderDevice,
    render_queue: &RenderQueue,
) -> BindGroup {
    let uniform = PulseRenderTargetUniform {
        width: render_target.width(),
        height: render_target.height(),
    };
    let uniform_buffer = create_uniform_buffer(
        uniform,
        Some("pulse_render_target_bind_group"),
        render_device,
        render_queue,
    );

    render_device.create_bind_group(
        Some("pulse_render_target_bind_group"),
        layout,
        &[
            // Uniform
            BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.into_binding(),
            },
            // Render target view
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(render_target.view()),
            },
        ],
    )
}
