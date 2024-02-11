use std::num::NonZeroU32;

use super::{PulseSettings, PULSE_GI_SHADER_HANDLE};
use crate::{scene::PulseSceneBindGroupLayout, PulseRenderTarget};
use bevy::{
    core_pipeline::{
        prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
        tonemapping::{DebandDither, Tonemapping},
    },
    pbr::{
        MeshPipeline, MeshPipelineKey, ScreenSpaceAmbientOcclusionSettings, ShadowFilteringMethod,
    },
    prelude::*,
    render::{
        render_asset::RenderAssets, render_resource::*, renderer::RenderDevice, view::ExtractedView,
    },
};

#[derive(Component)]
pub struct PulseGIPipeline {
    pub id: CachedComputePipelineId,
}

#[derive(Resource)]
pub struct PulseGILayout {
    pub mesh_pipeline: MeshPipeline,
    pub scene_layout: BindGroupLayout,
    pub view_layout: BindGroupLayout,
}

impl FromWorld for PulseGILayout {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let view_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                // GI texture
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: PulseRenderTarget::TEXTURE_FORMAT,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Shadow texture
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: PulseRenderTarget::TEXTURE_FORMAT,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Deferred texture sampler
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
                // Resolution uniform
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let mesh_pipeline = world.resource::<MeshPipeline>().clone();
        let scene_layout = world.resource::<PulseSceneBindGroupLayout>().0.clone();

        Self {
            mesh_pipeline,
            scene_layout,
            view_layout,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PulsePipelineKey {
    pub accumulate: bool,
    mesh_key: MeshPipelineKey,
}

impl SpecializedComputePipeline for PulseGILayout {
    type Key = PulsePipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        // NOTE: These shader defs aren't used by Pulse but are needed for various shader functions, eg. for unpacking the deferred texture.
        let mut shader_defs = Vec::new();

        let mesh_key = key.mesh_key;

        // Let the shader code know that it's running in a deferred pipeline.
        shader_defs.push("DEFERRED_LIGHTING_PIPELINE".into());

        if mesh_key.contains(MeshPipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());

            let method = mesh_key.intersection(MeshPipelineKey::TONEMAP_METHOD_RESERVED_BITS);

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
            if mesh_key.contains(MeshPipelineKey::DEBAND_DITHER) {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }

        if mesh_key.contains(MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION) {
            shader_defs.push("SCREEN_SPACE_AMBIENT_OCCLUSION".into());
        }

        if mesh_key.contains(MeshPipelineKey::ENVIRONMENT_MAP) {
            shader_defs.push("ENVIRONMENT_MAP".into());
        }

        if mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS) {
            shader_defs.push("NORMAL_PREPASS".into());
        }

        if mesh_key.contains(MeshPipelineKey::DEPTH_PREPASS) {
            shader_defs.push("DEPTH_PREPASS".into());
        }

        if mesh_key.contains(MeshPipelineKey::MOTION_VECTOR_PREPASS) {
            shader_defs.push("MOTION_VECTOR_PREPASS".into());
        }

        // Always true, since we're in the deferred lighting pipeline
        shader_defs.push("DEFERRED_PREPASS".into());

        let shadow_filter_method =
            mesh_key.intersection(MeshPipelineKey::SHADOW_FILTER_METHOD_RESERVED_BITS);
        if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_HARDWARE_2X2 {
            shader_defs.push("SHADOW_FILTER_METHOD_HARDWARE_2X2".into());
        } else if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_CASTANO_13 {
            shader_defs.push("SHADOW_FILTER_METHOD_CASTANO_13".into());
        } else if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_JIMENEZ_14 {
            shader_defs.push("SHADOW_FILTER_METHOD_JIMENEZ_14".into());
        }

        if key.accumulate {
            shader_defs.push("ACCUMULATE".into());
        }

        ComputePipelineDescriptor {
            label: Some("pulse_pipeline".into()),
            layout: vec![
                self.mesh_pipeline.get_view_layout(mesh_key.into()).clone(),
                self.scene_layout.clone(),
                self.view_layout.clone(),
            ],
            push_constant_ranges: vec![],
            shader: PULSE_GI_SHADER_HANDLE,
            shader_defs,
            entry_point: "gi".into(),
        }
    }
}

pub fn prepare_gi_pipelines(
    views: Query<
        (
            Entity,
            &ExtractedView,
            &PulseSettings,
            Option<&Tonemapping>,
            Option<&DebandDither>,
            Option<&EnvironmentMapLight>,
            Option<&ShadowFilteringMethod>,
            Option<&ScreenSpaceAmbientOcclusionSettings>,
            (
                Has<NormalPrepass>,
                Has<DepthPrepass>,
                Has<MotionVectorPrepass>,
            ),
        ),
        With<DeferredPrepass>,
    >,
    mut commands: Commands,
    mut pipelines: ResMut<SpecializedComputePipelines<PulseGILayout>>,
    layout: Res<PulseGILayout>,
    cache: Res<PipelineCache>,
    images: Res<RenderAssets<Image>>,
) {
    for (
        entity,
        view,
        pulse_settings,
        tonemapping,
        dither,
        environment_map,
        shadow_filter_method,
        ssao,
        (normal_prepass, depth_prepass, motion_vector_prepass),
    ) in &views
    {
        let mut mesh_key = MeshPipelineKey::from_hdr(view.hdr);

        if normal_prepass {
            mesh_key |= MeshPipelineKey::NORMAL_PREPASS;
        }

        if depth_prepass {
            mesh_key |= MeshPipelineKey::DEPTH_PREPASS;
        }

        if motion_vector_prepass {
            mesh_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }

        // Always true, since we're in the deferred lighting pipeline
        mesh_key |= MeshPipelineKey::DEFERRED_PREPASS;

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                mesh_key |= MeshPipelineKey::TONEMAP_IN_SHADER;
                mesh_key |= match tonemapping {
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
                mesh_key |= MeshPipelineKey::DEBAND_DITHER;
            }
        }

        if ssao.is_some() {
            mesh_key |= MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION;
        }

        let environment_map_loaded = match environment_map {
            Some(environment_map) => environment_map.is_loaded(&images),
            None => false,
        };
        if environment_map_loaded {
            mesh_key |= MeshPipelineKey::ENVIRONMENT_MAP;
        }

        match shadow_filter_method.unwrap_or(&ShadowFilteringMethod::default()) {
            ShadowFilteringMethod::Hardware2x2 => {
                mesh_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_HARDWARE_2X2;
            }
            ShadowFilteringMethod::Castano13 => {
                mesh_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_CASTANO_13;
            }
            ShadowFilteringMethod::Jimenez14 => {
                mesh_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_JIMENEZ_14;
            }
        }

        let view_key = PulsePipelineKey {
            accumulate: pulse_settings.accumulate,
            mesh_key,
        };

        let id = pipelines.specialize(&cache, &layout, view_key);
        commands.entity(entity).insert(PulseGIPipeline { id });
    }
}
