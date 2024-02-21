use super::{PulsePathTracerCamera, PULSE_PATH_TRACER_SHADER_HANDLE};
use crate::scene::PulseSceneBindGroupLayout;
use bevy::{
    core_pipeline::{
        prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
        tonemapping::{DebandDither, Tonemapping},
    },
    pbr::{
        irradiance_volume::IrradianceVolume, MeshPipeline, MeshPipelineKey, RenderViewLightProbes,
        ScreenSpaceAmbientOcclusionSettings, ShadowFilteringMethod,
    },
    prelude::*,
    render::{
        render_asset::RenderAssets, render_resource::*, renderer::RenderDevice, view::ExtractedView,
    },
};

#[derive(Component)]
pub struct PulsePathTracerPipeline {
    pub id: CachedComputePipelineId,
}

#[derive(Resource)]
pub struct PulsePathTracerLayout {
    pub mesh_pipeline: MeshPipeline,
    pub scene_layout: BindGroupLayout,
    pub view_layout: BindGroupLayout,
}

impl FromWorld for PulsePathTracerLayout {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let view_layout = device.create_bind_group_layout(
            None,
            &[
                // Output texture
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: TextureFormat::Rgba32Float,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Path tracer uniform
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        );

        let mesh_pipeline = world.resource::<MeshPipeline>().clone();
        let scene_layout = world.resource::<PulseSceneBindGroupLayout>().0.clone();

        Self {
            mesh_pipeline,
            scene_layout,
            view_layout,
        }
    }
}

impl SpecializedComputePipeline for PulsePathTracerLayout {
    type Key = MeshPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        // NOTE: These shader defs aren't used by Pulse but are needed for various shader functions, eg. for unpacking the deferred texture.
        let mut shader_defs = Vec::new();

        // Let the shader code know that it's running in a deferred pipeline.
        shader_defs.push("DEFERRED_LIGHTING_PIPELINE".into());

        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        shader_defs.push("WEBGL2".into());

        if key.contains(MeshPipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());

            let method = key.intersection(MeshPipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            if method == MeshPipelineKey::TONEMAP_METHOD_NONE {
                shader_defs.push("TONEMAP_METHOD_NONE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD {
                shader_defs.push("TONEMAP_METHOD_REINHARD".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_ACES_FITTED {
                shader_defs.push("TONEMAP_METHOD_ACES_FITTED ".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_AGX {
                shader_defs.push("TONEMAP_METHOD_AGX".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM {
                shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC {
                shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE {
                shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into());
            }

            // Debanding is tied to tonemapping in the shader, cannot run without it.
            if key.contains(MeshPipelineKey::DEBAND_DITHER) {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }

        if key.contains(MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION) {
            shader_defs.push("SCREEN_SPACE_AMBIENT_OCCLUSION".into());
        }

        if key.contains(MeshPipelineKey::ENVIRONMENT_MAP) {
            shader_defs.push("ENVIRONMENT_MAP".into());
        }

        if key.contains(MeshPipelineKey::IRRADIANCE_VOLUME) {
            shader_defs.push("IRRADIANCE_VOLUME".into());
        }

        if key.contains(MeshPipelineKey::NORMAL_PREPASS) {
            shader_defs.push("NORMAL_PREPASS".into());
        }

        if key.contains(MeshPipelineKey::DEPTH_PREPASS) {
            shader_defs.push("DEPTH_PREPASS".into());
        }

        if key.contains(MeshPipelineKey::MOTION_VECTOR_PREPASS) {
            shader_defs.push("MOTION_VECTOR_PREPASS".into());
        }

        // Always true, since we're in the deferred lighting pipeline
        shader_defs.push("DEFERRED_PREPASS".into());

        let shadow_filter_method =
            key.intersection(MeshPipelineKey::SHADOW_FILTER_METHOD_RESERVED_BITS);
        if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_HARDWARE_2X2 {
            shader_defs.push("SHADOW_FILTER_METHOD_HARDWARE_2X2".into());
        } else if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_CASTANO_13 {
            shader_defs.push("SHADOW_FILTER_METHOD_CASTANO_13".into());
        } else if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_JIMENEZ_14 {
            shader_defs.push("SHADOW_FILTER_METHOD_JIMENEZ_14".into());
        }

        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        shader_defs.push("SIXTEEN_BYTE_ALIGNMENT".into());

        ComputePipelineDescriptor {
            label: Some("pulse_path_tracer_pipeline".into()),
            layout: vec![
                self.mesh_pipeline.get_view_layout(key.into()).clone(),
                self.scene_layout.clone(),
                self.view_layout.clone(),
            ],
            push_constant_ranges: vec![],
            shader: PULSE_PATH_TRACER_SHADER_HANDLE,
            shader_defs,
            entry_point: "path_trace".into(),
        }
    }
}

pub fn prepare_path_tracer_pipelines(
    views: Query<
        (
            Entity,
            &ExtractedView,
            &PulsePathTracerCamera,
            Option<&Tonemapping>,
            Option<&DebandDither>,
            Option<&ShadowFilteringMethod>,
            Has<ScreenSpaceAmbientOcclusionSettings>,
            (
                Has<NormalPrepass>,
                Has<DepthPrepass>,
                Has<MotionVectorPrepass>,
            ),
            Has<RenderViewLightProbes<EnvironmentMapLight>>,
            Has<RenderViewLightProbes<IrradianceVolume>>,
        ),
        With<DeferredPrepass>,
    >,
    mut commands: Commands,
    mut pipelines: ResMut<SpecializedComputePipelines<PulsePathTracerLayout>>,
    layout: Res<PulsePathTracerLayout>,
    cache: Res<PipelineCache>,
    images: Res<RenderAssets<Image>>,
) {
    for (
        entity,
        view,
        path_tracer,
        tonemapping,
        dither,
        shadow_filter_method,
        ssao,
        (normal_prepass, depth_prepass, motion_vector_prepass),
        has_environment_maps,
        has_irradiance_volumes,
    ) in &views
    {
        let mut mesh_view_key = MeshPipelineKey::from_hdr(view.hdr);

        if normal_prepass {
            mesh_view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }

        if depth_prepass {
            mesh_view_key |= MeshPipelineKey::DEPTH_PREPASS;
        }

        if motion_vector_prepass {
            mesh_view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }

        // Always true, since we're in the deferred lighting pipeline
        mesh_view_key |= MeshPipelineKey::DEFERRED_PREPASS;

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                mesh_view_key |= MeshPipelineKey::TONEMAP_IN_SHADER;
                mesh_view_key |= match tonemapping {
                    Tonemapping::None => MeshPipelineKey::TONEMAP_METHOD_NONE,
                    Tonemapping::Reinhard => MeshPipelineKey::TONEMAP_METHOD_REINHARD,
                    Tonemapping::ReinhardLuminance => {
                        MeshPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE
                    }
                    Tonemapping::AcesFitted => MeshPipelineKey::TONEMAP_METHOD_ACES_FITTED,
                    Tonemapping::AgX => MeshPipelineKey::TONEMAP_METHOD_AGX,
                    Tonemapping::SomewhatBoringDisplayTransform => {
                        MeshPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
                    }
                    Tonemapping::TonyMcMapface => MeshPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE,
                    Tonemapping::BlenderFilmic => MeshPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC,
                };
            }
            if let Some(DebandDither::Enabled) = dither {
                mesh_view_key |= MeshPipelineKey::DEBAND_DITHER;
            }
        }

        if ssao {
            mesh_view_key |= MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION;
        }

        // We don't need to check to see whether the environment map is loaded
        // because [`gather_light_probes`] already checked that for us before
        // adding the [`RenderViewEnvironmentMaps`] component.
        if has_environment_maps {
            mesh_view_key |= MeshPipelineKey::ENVIRONMENT_MAP;
        }

        if has_irradiance_volumes {
            mesh_view_key |= MeshPipelineKey::IRRADIANCE_VOLUME;
        }

        match shadow_filter_method.unwrap_or(&ShadowFilteringMethod::default()) {
            ShadowFilteringMethod::Hardware2x2 => {
                mesh_view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_HARDWARE_2X2;
            }
            ShadowFilteringMethod::Castano13 => {
                mesh_view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_CASTANO_13;
            }
            ShadowFilteringMethod::Jimenez14 => {
                mesh_view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_JIMENEZ_14;
            }
        }

        let id = pipelines.specialize(&cache, &layout, mesh_view_key);
        commands
            .entity(entity)
            .insert(PulsePathTracerPipeline { id });
    }
}
